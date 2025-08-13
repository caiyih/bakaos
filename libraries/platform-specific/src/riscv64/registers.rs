macro_rules! prop_general_reg {
    ($reg:ident) => {
        #[inline(always)]
        pub fn $reg() -> usize {
            let v;
            unsafe {
                ::core::arch::asm!(
                    ::core::concat!("mv {0}, ", ::core::stringify!($reg)),
                    out(reg) v
                );
            }
            v
        }
    };
}

prop_general_reg!(x0);
prop_general_reg!(ra);
prop_general_reg!(sp);
prop_general_reg!(gp);
prop_general_reg!(tp);
prop_general_reg!(t0);
prop_general_reg!(t1);
prop_general_reg!(t2);
prop_general_reg!(fp);
prop_general_reg!(s1);
prop_general_reg!(a0);
prop_general_reg!(a1);
prop_general_reg!(a2);
prop_general_reg!(a3);
prop_general_reg!(a4);
prop_general_reg!(a5);
prop_general_reg!(a6);
prop_general_reg!(a7);
prop_general_reg!(s2);
prop_general_reg!(s3);
prop_general_reg!(s4);
prop_general_reg!(s5);
prop_general_reg!(s6);
prop_general_reg!(s7);
prop_general_reg!(s8);
prop_general_reg!(s9);
prop_general_reg!(s10);
prop_general_reg!(s11);
prop_general_reg!(t3);
prop_general_reg!(t4);
prop_general_reg!(t5);
prop_general_reg!(t6);

macro_rules! prop_privileged_reg {
    ($reg:ident) => {
        #[inline(always)]
        pub fn $reg() -> usize {
            let v;
            unsafe {
                ::core::arch::asm!(
                    ::core::concat!("csrr {0}, ", ::core::stringify!($reg)),
                    out(reg) v
                );
            }
            v
        }
    };
}

prop_privileged_reg!(time);
prop_privileged_reg!(satp);
