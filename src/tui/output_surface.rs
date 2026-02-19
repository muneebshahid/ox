#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct OutputViewport {
    pub(in crate::tui) x: u16,
    pub(in crate::tui) y: u16,
    pub(in crate::tui) width: u16,
    pub(in crate::tui) height: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) struct CellPos {
    pub(in crate::tui) col: u16,
    pub(in crate::tui) row: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) struct OutputRenderSnapshot {
    pub(in crate::tui) viewport: OutputViewport,
    pub(in crate::tui) cells: Vec<Vec<String>>,
}
