macro_rules! prop_general_reg {
    ($reg:ident) => {
        #[inline(always)]
        pub fn $reg() -> usize {
            let v: usize;
            unsafe {
                ::core::arch::asm!(
                    concat!("move {}, $", stringify!($reg)), out(reg) v);
            }
            v
        }
    };
}

prop_general_reg!(r0);
prop_general_reg!(r1);
prop_general_reg!(r2);
prop_general_reg!(r3);
prop_general_reg!(r4);
prop_general_reg!(r5);
prop_general_reg!(r6);
prop_general_reg!(r7);
prop_general_reg!(r8);
prop_general_reg!(r9);
prop_general_reg!(r10);
prop_general_reg!(r11);
prop_general_reg!(r12);
prop_general_reg!(r13);
prop_general_reg!(r14);
prop_general_reg!(r15);
prop_general_reg!(r16);
prop_general_reg!(r17);
prop_general_reg!(r18);
prop_general_reg!(r19);
prop_general_reg!(r20);
prop_general_reg!(r21);
prop_general_reg!(r22);
prop_general_reg!(r23);
prop_general_reg!(r24);
prop_general_reg!(r25);
prop_general_reg!(r26);
prop_general_reg!(r27);
prop_general_reg!(r28);
prop_general_reg!(r29);
prop_general_reg!(r30);
prop_general_reg!(r31);

// aliases for general registers

// r0
prop_general_reg!(zero);

// r1
prop_general_reg!(ra);

// r2
prop_general_reg!(tp);

// r3
prop_general_reg!(sp);

// r4 - r11 (a0 - a7)
prop_general_reg!(a0);
prop_general_reg!(a1);
prop_general_reg!(a2);
prop_general_reg!(a3);
prop_general_reg!(a4);
prop_general_reg!(a5);
prop_general_reg!(a6);
prop_general_reg!(a7);

// r4 - r5 (v0 - v1)
prop_general_reg!(v0);
prop_general_reg!(v1);

// r12 - r20 (t0 - t8)
prop_general_reg!(t0);
prop_general_reg!(t1);
prop_general_reg!(t2);
prop_general_reg!(t3);
prop_general_reg!(t4);
prop_general_reg!(t5);
prop_general_reg!(t6);
prop_general_reg!(t7);
prop_general_reg!(t8);

// r22
prop_general_reg!(fp);

// r23 - r31 (s0 - s8)
prop_general_reg!(s0);
prop_general_reg!(s1);
prop_general_reg!(s2);
prop_general_reg!(s3);
prop_general_reg!(s4);
prop_general_reg!(s5);
prop_general_reg!(s6);
prop_general_reg!(s7);
prop_general_reg!(s8);

macro_rules! prop_privileged_reg {
    ($reg:ident, $csr:literal) => {
        #[inline(always)]
        pub fn $reg() -> usize {
            let v;
            unsafe {
                ::core::arch::asm!(
                    ::core::concat!("csrrd {0}, ", ::core::stringify!($csr)),
                    out(reg) v
                );
            }
            v
        }
    };
}

prop_privileged_reg!(pgdh, 0x1a);
prop_privileged_reg!(pgdl, 0x19);
