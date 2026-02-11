use tokio::sync::broadcast;

use crate::events::types::CoreEvent;

#[derive(Clone)]
pub struct EventHub {
    tx: broadcast::Sender<CoreEvent>,
}

impl EventHub {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "event hub capacity must be > 0");
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: CoreEvent) -> usize {
        self.tx.send(event).unwrap_or(0)
    }

    pub fn subscribe(&self) -> Subscription {
        Subscription {
            rx: self.tx.subscribe(),
        }
    }
}

pub struct Subscription {
    rx: broadcast::Receiver<CoreEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RecvError {
    Lagged(u64),
    Closed,
}

impl Subscription {
    pub async fn recv(&mut self) -> Result<CoreEvent, RecvError> {
        match self.rx.recv().await {
            Ok(event) => Ok(event),
            Err(broadcast::error::RecvError::Lagged(n)) => Err(RecvError::Lagged(n)),
            Err(broadcast::error::RecvError::Closed) => Err(RecvError::Closed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{EventHub, RecvError};
    use crate::events::types::CoreEvent;

    #[tokio::test]
    async fn publishes_to_all_subscribers() {
        let hub = EventHub::new(8);
        let mut sub_a = hub.subscribe();
        let mut sub_b = hub.subscribe();

        let delivered = hub.publish(CoreEvent::Tick);
        assert_eq!(delivered, 2);

        assert_eq!(sub_a.recv().await, Ok(CoreEvent::Tick));
        assert_eq!(sub_b.recv().await, Ok(CoreEvent::Tick));
    }

    #[tokio::test]
    async fn reports_lag_for_slow_subscribers() {
        let hub = EventHub::new(2);
        let mut sub = hub.subscribe();

        hub.publish(CoreEvent::AgentTextDelta("a".to_string()));
        hub.publish(CoreEvent::AgentTextDelta("b".to_string()));
        hub.publish(CoreEvent::AgentTextDelta("c".to_string()));

        assert_eq!(sub.recv().await, Err(RecvError::Lagged(1)));
        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentTextDelta("b".to_string()))
        );
        assert_eq!(
            sub.recv().await,
            Ok(CoreEvent::AgentTextDelta("c".to_string()))
        );
    }

    #[tokio::test]
    async fn receiver_closes_when_all_senders_are_dropped() {
        let hub = EventHub::new(4);
        let mut sub = hub.subscribe();

        hub.publish(CoreEvent::Tick);
        drop(hub);

        assert_eq!(sub.recv().await, Ok(CoreEvent::Tick));
        assert_eq!(sub.recv().await, Err(RecvError::Closed));
    }
}
