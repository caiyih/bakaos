use crate::pt::IPageTableArchAttribute;

pub struct SV39PageTableAttribute;

impl IPageTableArchAttribute for SV39PageTableAttribute {
    const LEVELS: usize = 3;
    const PA_MAX_BITS: usize = 56;
    const VA_MAX_BITS: usize = 39;
}
