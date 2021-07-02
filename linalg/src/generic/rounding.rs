use crate::frame::mmm::*;

pub trait PseudoRightShift {
    fn q_away(self, mult: Self, shift: usize) -> Self;
    fn q_even(self, mult: Self, shift: usize) -> Self;
    fn q_to_plus_inf(self, mult: Self, shift: usize) -> Self;
    fn q_scale(self, mult: i32, shift: usize, policy: RoundingPolicy) -> Self;
}

impl PseudoRightShift for f32 {
    fn q_even(self, mult: Self, shift: usize) -> Self {
        self * mult * 2f32.powi(-(shift as i32))
    }
    fn q_to_plus_inf(self, mult: Self, shift: usize) -> Self {
        self * mult * 2f32.powi(-(shift as i32))
    }
    fn q_away(self, mult: Self, shift: usize) -> Self {
        self * mult * 2f32.powi(-(shift as i32))
    }

    fn q_scale(self, mult: i32, shift: usize, _policy: RoundingPolicy) -> Self {
        self * mult as f32 * 2. * 2f32.powi(-(shift as i32))
    }
}

impl PseudoRightShift for i32 {
    fn q_even(self, mult: Self, shift: usize) -> Self {
        let v = ((self as i64 * mult as i64) >> (30 + shift)) as i32;
        let truncated = v.abs();
        let nudge = ((truncated & 0x3) == 0x3) as usize as i32;
        let pos = (truncated + nudge) >> 1;
        if v.is_negative() {
            -pos
        } else {
            pos
        }
    }
    fn q_to_plus_inf(self, mult: Self, shift: usize) -> Self {
        let v = ((self as i64 * mult as i64) >> (30 + shift)) as i32;
        (v + 1) >> 1
    }
    fn q_away(self, mult: Self, shift: usize) -> Self {
        let v = ((self.abs() as i64 * mult as i64) >> (30 + shift)) as i32;
        ((v + 1) >> 1) * self.signum()
    }
    fn q_scale(self, mult: i32, shift: usize, policy: RoundingPolicy) -> Self {
        use RoundingPolicy::*;
        let val = self as i64 * mult as i64;
        let shift = shift + 31;
        let nudge1 = 1 << (shift - 1);
        let nudge2 = (1 << (shift - 1)) - 1;
        (match policy {
            Zero => val.signum() * (val.abs() + nudge2 >> shift),
            MinusInf => {
                let nudge = if val < 0 { nudge1 } else { nudge2 };
                val.signum() * (val.abs() + nudge >> shift)
            }
            PlusInf => {
                let nudge = if val < 0 { nudge2 } else { nudge1 };
                val.signum() * (val.abs() + nudge >> shift)
            }
            Away => val.signum() * (val.abs() + nudge1 >> shift),
            Even => {
                let nudge = if (val.abs() >> shift) & 0x1 == 0x1 { nudge1 } else { nudge2 };
                (val.abs() + nudge >> shift) * val.signum()
            }
            Odd => {
                let nudge = if (val.abs() >> shift) & 0x1 == 0x0 { nudge1 } else { nudge2 };
                (val.abs() + nudge >> shift) * val.signum()
            }
            _ => panic!(),
        }) as i32
    }
}

#[cfg(test)]
mod test {
    use super::RoundingPolicy::*;
    use super::*;

    #[test]
    fn test_zero() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, Zero), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, Zero), 0);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, Zero), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, Zero), 1);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, Zero), 0);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, Zero), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, Zero), -1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, Zero), 0);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, Zero), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, Zero), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, Zero), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, Zero), 1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, Zero), 0);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, Zero), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, Zero), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, Zero), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, Zero), -1);
    }

    #[test]
    fn test_away() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, Away), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, Away), 1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, Away), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, Away), 2);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, Away), -1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, Away), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, Away), -2);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, Away), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, Away), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, Away), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, Away), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, Away), 2);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, Away), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, Away), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, Away), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, Away), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, Away), -2);
    }

    #[test]
    fn test_plus_inf() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, PlusInf), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, PlusInf), 1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, PlusInf), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, PlusInf), 2);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, PlusInf), 0);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, PlusInf), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, PlusInf), -1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, PlusInf), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, PlusInf), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, PlusInf), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, PlusInf), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, PlusInf), 2);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, PlusInf), 0);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, PlusInf), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, PlusInf), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, PlusInf), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, PlusInf), -1);
    }

    #[test]
    fn test_minus_inf() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, MinusInf), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, MinusInf), 0);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, MinusInf), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, MinusInf), 1);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, MinusInf), -1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, MinusInf), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, MinusInf), -2);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, MinusInf), 0);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, MinusInf), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, MinusInf), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, MinusInf), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, MinusInf), 1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, MinusInf), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, MinusInf), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, MinusInf), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, MinusInf), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, MinusInf), -2);
        assert_eq!((-9i32).q_scale(2i32.pow(30), 5, MinusInf), 0);
    }

    #[test]
    fn test_even() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, Even), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, Even), 0);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, Even), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, Even), 2);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, Even), 0);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, Even), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, Even), -2);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, Even), 0);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, Even), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, Even), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, Even), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, Even), 2);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, Even), 0);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, Even), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, Even), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, Even), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, Even), -2);
    }

    #[test]
    fn test_odd() {
        assert_eq!(0i32.q_scale(2i32.pow(30), 0, Odd), 0);
        assert_eq!(1i32.q_scale(2i32.pow(30), 0, Odd), 1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 0, Odd), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 0, Odd), 1);
        assert_eq!((-1i32).q_scale(2i32.pow(30), 0, Odd), -1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 0, Odd), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 0, Odd), -1);
        assert_eq!(2i32.q_scale(2i32.pow(30), 1, Odd), 1);
        assert_eq!(3i32.q_scale(2i32.pow(30), 1, Odd), 1);
        assert_eq!(4i32.q_scale(2i32.pow(30), 1, Odd), 1);
        assert_eq!(5i32.q_scale(2i32.pow(30), 1, Odd), 1);
        assert_eq!(6i32.q_scale(2i32.pow(30), 1, Odd), 1);
        assert_eq!((-2i32).q_scale(2i32.pow(30), 1, Odd), -1);
        assert_eq!((-3i32).q_scale(2i32.pow(30), 1, Odd), -1);
        assert_eq!((-4i32).q_scale(2i32.pow(30), 1, Odd), -1);
        assert_eq!((-5i32).q_scale(2i32.pow(30), 1, Odd), -1);
        assert_eq!((-6i32).q_scale(2i32.pow(30), 1, Odd), -1);
    }
}
