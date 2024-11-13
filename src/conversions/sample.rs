use cpal::{FromSample, Sample as CpalSample};
use std::marker::PhantomData;

/// Converts the samples data type to `O`.
#[derive(Clone, Debug)]
pub struct DataConverter<I, O> {
    input: I,
    marker: PhantomData<O>,
}

impl<I, O> DataConverter<I, O> {
    /// Builds a new converter.
    #[inline]
    pub fn new(input: I) -> DataConverter<I, O> {
        DataConverter {
            input,
            marker: PhantomData,
        }
    }

    /// Destroys this iterator and returns the underlying iterator.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input
    }

    /// get mutable access to the iterator
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }
}

impl<I, O> Iterator for DataConverter<I, O>
where
    I: Iterator,
    I::Item: Sample,
    O: FromSample<I::Item> + Sample,
{
    type Item = O;

    #[inline]
    fn next(&mut self) -> Option<O> {
        self.input.next().map(|s| CpalSample::from_sample(s))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I, O> ExactSizeIterator for DataConverter<I, O>
where
    I: ExactSizeIterator,
    I::Item: Sample,
    O: FromSample<I::Item> + Sample,
{
}

/// Represents a value of a single sample.
///
/// This trait is implemented by default on three types: `i16`, `u16` and `f32`.
///
/// - For `i16`, silence corresponds to the value `0`. The minimum and maximum amplitudes are
///   represented by `i16::min_value()` and `i16::max_value()` respectively.
/// - For `u16`, silence corresponds to the value `u16::max_value() / 2`. The minimum and maximum
///   amplitudes are represented by `0` and `u16::max_value()` respectively.
/// - For `f32`, silence corresponds to the value `0.0`. The minimum and maximum amplitudes are
///   represented by `-1.0` and `1.0` respectively.
///
/// You can implement this trait on your own type as well if you wish so.
///
pub trait Sample: CpalSample {
    /// Linear interpolation between two samples.
    ///
    /// The result should be equvivalent to
    /// `first * (1 - numerator / denominator) + second * numerator / denominator`.
    fn lerp(first: Self, second: Self, numerator: u32, denominator: u32) -> Self;
    /// Multiplies the value of this sample by the given amount.
    fn amplify(self, value: f32) -> Self;

    /// Converts the sample to a f32 value.
    fn to_f32(self) -> f32;

    /// Calls `saturating_add` on the sample.
    fn saturating_add(self, other: Self) -> Self;

    /// Returns the value corresponding to the absence of sound.
    fn zero_value() -> Self;
}

impl Sample for u16 {
    #[inline]
    fn lerp(first: u16, second: u16, numerator: u32, denominator: u32) -> u16 {
        let d =
            first as i64 + (second as i64 - first as i64) * numerator as i64 / denominator as i64;
        u16::try_from(d).expect("numerator / denominator is within [0, 1] range")
    }

    #[inline]
    fn amplify(self, value: f32) -> u16 {
        ((self as f32) * value) as u16
    }

    #[inline]
    fn to_f32(self) -> f32 {
        // Convert u16 to f32 in the range [-1.0, 1.0]
        (self as f32 - 32768.0) / 32768.0
    }

    #[inline]
    fn saturating_add(self, other: u16) -> u16 {
        self.saturating_add(other)
    }

    #[inline]
    fn zero_value() -> u16 {
        32768
    }
}

impl Sample for i16 {
    #[inline]
    fn lerp(first: i16, second: i16, numerator: u32, denominator: u32) -> i16 {
        let d =
            first as i64 + (second as i64 - first as i64) * numerator as i64 / denominator as i64;
        i16::try_from(d).expect("numerator / denominator is within [0, 1] range")
    }

    #[inline]
    fn amplify(self, value: f32) -> i16 {
        ((self as f32) * value) as i16
    }

    #[inline]
    fn to_f32(self) -> f32 {
        // Convert i16 to f32 in the range [-1.0, 1.0]
        self as f32 / 32768.0
    }

    #[inline]
    fn saturating_add(self, other: i16) -> i16 {
        self.saturating_add(other)
    }

    #[inline]
    fn zero_value() -> i16 {
        0
    }
}

impl Sample for f32 {
    #[inline]
    fn lerp(first: f32, second: f32, numerator: u32, denominator: u32) -> f32 {
        first + (second - first) * numerator as f32 / denominator as f32
    }

    #[inline]
    fn amplify(self, value: f32) -> f32 {
        self * value
    }

    #[inline]
    fn to_f32(self) -> f32 {
        // f32 is already in the correct format
        self
    }

    #[inline]
    fn saturating_add(self, other: f32) -> f32 {
        self + other
    }

    #[inline]
    fn zero_value() -> f32 {
        0.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    #[test]
    fn lerp_u16_constraints() {
        let a = 12u16;
        let b = 31u16;
        assert_eq!(Sample::lerp(a, b, 0, 1), a);
        assert_eq!(Sample::lerp(a, b, 1, 1), b);

        assert_eq!(Sample::lerp(0, u16::MAX, 0, 1), 0);
        assert_eq!(Sample::lerp(0, u16::MAX, 1, 1), u16::MAX);
        // Zeroes
        assert_eq!(Sample::lerp(0u16, 0, 0, 1), 0);
        assert_eq!(Sample::lerp(0u16, 0, 1, 1), 0);
        // Downward changes
        assert_eq!(Sample::lerp(1u16, 0, 0, 1), 1);
        assert_eq!(Sample::lerp(1u16, 0, 1, 1), 0);
    }

    #[test]
    #[should_panic]
    fn lerp_u16_overflow() {
        Sample::lerp(0u16, 1, u16::MAX as u32 + 1, 1);
    }

    #[test]
    fn lerp_i16_constraints() {
        let a = 12i16;
        let b = 31i16;
        assert_eq!(Sample::lerp(a, b, 0, 1), a);
        assert_eq!(Sample::lerp(a, b, 1, 1), b);

        assert_eq!(Sample::lerp(0, i16::MAX, 0, 1), 0);
        assert_eq!(Sample::lerp(0, i16::MAX, 1, 1), i16::MAX);
        assert_eq!(Sample::lerp(0, i16::MIN, 1, 1), i16::MIN);
        // Zeroes
        assert_eq!(Sample::lerp(0u16, 0, 0, 1), 0);
        assert_eq!(Sample::lerp(0u16, 0, 1, 1), 0);
        // Downward changes
        assert_eq!(Sample::lerp(a, i16::MIN, 0, 1), a);
        assert_eq!(Sample::lerp(a, i16::MIN, 1, 1), i16::MIN);
    }

    #[test]
    #[should_panic]
    fn lerp_i16_overflow_max() {
        Sample::lerp(0i16, 1, i16::MAX as u32 + 1, 1);
    }

    #[test]
    #[should_panic]
    fn lerp_i16_overflow_min() {
        Sample::lerp(0i16, -1, (i16::MIN.abs() + 1) as u32, 1);
    }

    quickcheck! {
        fn lerp_u16_random(first: u16, second: u16, numerator: u32, denominator: u32) -> TestResult {
            if denominator == 0 { return TestResult::discard(); }
            let a = first as f64;
            let b = second as f64;
            let c = numerator as f64 / denominator as f64;
            if c < 0.0 || c > 1.0 { return TestResult::discard(); };
            let reference = a + (b - a) * c;
            let x = Sample::lerp(first, second, numerator, denominator) as f64;
            let d = x - reference;
            TestResult::from_bool(d.abs() < 1.0)
        }
    }
}
