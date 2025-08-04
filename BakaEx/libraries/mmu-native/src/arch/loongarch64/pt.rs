use crate::IPageTableArchAttribute;

pub struct LA64PageTableAttribute;

impl IPageTableArchAttribute for LA64PageTableAttribute {
    const LEVELS: usize = 4;
    const PA_MAX_BITS: usize = 48;
    const VA_MAX_BITS: usize = 48;
}
