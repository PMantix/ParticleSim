#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoilLinkType {
    Same,
    Opposite,
}

#[derive(Clone, Copy, Debug)]
pub struct FoilLink {
    pub a: usize,
    pub b: usize,
    pub link_type: FoilLinkType,
}
