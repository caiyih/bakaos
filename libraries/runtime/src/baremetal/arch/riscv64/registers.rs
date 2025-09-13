pub mod gr {
    macro_rules! define_gr {
        () => ();

        ($name:tt, $($rest:tt)*) => {
            define_gr!($name);
            define_gr!($($rest)*);
        };

        ($name:tt) => {
            paste::paste! {
                #[inline(always)]
                pub fn [<get_ $name>]() -> usize {
                    let val;
                    unsafe {
                        ::core::arch::asm!(concat!("mv {}, ", stringify!($name)), out(reg) val);
                    }
                    val
                }

                #[inline(always)]
                pub unsafe fn [<set_ $name>](val: usize) {
                    ::core::arch::asm!(concat!("mv ", stringify!($name), ", {}"), in(reg) val);
                }
            }
        };
    }

    define_gr!(x0, zero);
    define_gr!(x1, ra);
    define_gr!(x2, sp);
    define_gr!(x3, gp);
    define_gr!(x4, tp);
    define_gr!(x5, t0);
    define_gr!(x6, t1);
    define_gr!(x7, t2);
    define_gr!(x8, s0, fp);
    define_gr!(x9, s1);
    define_gr!(x10, a0);
    define_gr!(x11, a1);
    define_gr!(x12, a2);
    define_gr!(x13, a3);
    define_gr!(x14, a4);
    define_gr!(x15, a5);
    define_gr!(x16, a6);
    define_gr!(x17, a7);
    define_gr!(x18, s2);
    define_gr!(x19, s3);
    define_gr!(x20, s4);
    define_gr!(x21, s5);
    define_gr!(x22, s6);
    define_gr!(x23, s7);
    define_gr!(x24, s8);
    define_gr!(x25, s9);
    define_gr!(x26, s10);
    define_gr!(x27, s11);
    define_gr!(x28, t3);
    define_gr!(x29, t4);
    define_gr!(x30, t5);
    define_gr!(x31, t6);
}
