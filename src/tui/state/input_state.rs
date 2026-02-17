use super::input_buffer::InputBuffer;

pub(super) struct InputState {
    pub(super) buffer: InputBuffer, // Editable input text and cursor byte-offset state.
}

impl InputState {
    pub(super) fn new() -> Self {
        Self {
            buffer: InputBuffer::new(),
        }
    }
}
