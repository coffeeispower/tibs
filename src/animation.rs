//! # Animation Module
//!
//! This module provides a high-level API for creating and coordinating animations in Rust.
//! It defines the core `Animation` trait, which abstracts the behavior common to all animations.
//! An animation is expected to:
//!
//! - Update its state based on a time delta (in seconds), returning a vector of tuples `(id, progress)`,
//!   where `progress` is a value between 0.0 (start) and 1.0 (complete).
//! - Report whether it has started or finished.
//! - Be reset to its initial state.
//!
//! The module includes several concrete implementations of the `Animation` trait:
//!
//! - **BasicAnimation**: A simple animation that runs for a specified duration and applies a custom easing function.
//! - **SequenceAnimation**: Plays a series of animations sequentially (one after the other). Only the currently active
//!   animation reports its progress.
//! - **ParallelAnimation**: Runs multiple animations concurrently, returning the progress of each animation simultaneously.
//! - **DelayAnimation**: Wraps another animation and introduces a delay before it starts updating.
//! - **LoopingAnimation**: Repeats an animation a fixed number of times or infinitely (if repeat count is zero).
//! - **BackAndForthAnimation**: Plays an animation forward and then in reverse, repeating for a specified number of cycles.
//!
//! Additionally, the module provides helper macros for grouping animations:
//!
//! - `seq!`: Constructs a `SequenceAnimation` from a list of animations.
//! - `all!`: Constructs a `ParallelAnimation` from a list of animations.
//!
//! ## Example Usage
//!
//! ```rust
//! use tibs::animation::{Animation, BasicAnimation, SequenceAnimation, ParallelAnimation, DelayAnimation};
//! use tibs::animation::easing::{linear, ease_in_out_cubic};
//! use tibs::{seq, all};
//!
//! // Create basic animations
//! let anim1 = BasicAnimation::new("fade_in", 2.0, linear);
//! let anim2 = BasicAnimation::new("move", 3.0, ease_in_out_cubic);
//!
//! // Group animations in sequence
//! let mut sequence = SequenceAnimation::new(vec![Box::new(anim1), Box::new(anim2)]);
//!
//! // Or using the provided macro:
//! let mut sequence_macro = seq!(
//!     BasicAnimation::new("fade_in", 2.0, linear),
//!     BasicAnimation::new("move", 3.0, ease_in_out_cubic)
//! );
//!
//! // Create a parallel group using the `all!` macro
//! let mut parallel = all!(
//!     BasicAnimation::new("animA", 2.5, linear),
//!     BasicAnimation::new("animB", 2.5, ease_in_out_cubic)
//! );
//!
//! // Create a delayed animation that starts after 1 second
//! let mut delayed = DelayAnimation::new(1.0, BasicAnimation::new("delayed", 2.0, linear));
//!
//! // In your render loop, update the animations with a delta time (e.g., 0.1 seconds)
//! let updates = sequence.update(0.1);
//! for (id, progress) in updates {
//!     println!("Animation '{}' progress: {:.2}", id, progress);
//! }
//! ```

use std::sync::mpsc::{self, Receiver, Sender};

pub trait Animation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)>;
    fn is_finished(&self) -> bool;
    fn has_started(&self) -> bool;
    fn reset(&mut self);
}

pub struct BasicAnimation<F = fn(f32) -> f32>
where
    F: Fn(f32) -> f32,
{
    id: String,
    duration: f32,
    elapsed: f32,
    easing: F,
}

impl<F> BasicAnimation<F>
where
    F: Fn(f32) -> f32,
{
    pub fn new(id: &str, duration: f32, easing: F) -> Self {
        BasicAnimation {
            id: id.to_string(),
            duration,
            elapsed: 0.0,
            easing,
        }
    }
}

impl<F> Animation for BasicAnimation<F>
where
    F: Fn(f32) -> f32,
{
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        if self.is_finished() && delta.is_sign_positive() {
            return vec![(self.id.clone(), 1.0)];
        }
        let delta = if self.elapsed + delta < 0.0 {
            -self.elapsed
        } else {
            delta
        };
        self.elapsed += delta;
        if self.elapsed > self.duration {
            self.elapsed = self.duration;
        }
        let progress = (self.elapsed / self.duration).min(1.0);
        let eased_progress = (self.easing)(progress);
        vec![(self.id.clone(), eased_progress)]
    }

    fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    fn has_started(&self) -> bool {
        self.elapsed > 0.0
    }
    fn reset(&mut self) {
        self.elapsed = 0.0;
    }
}

pub struct SequenceAnimation {
    animations: Vec<Box<dyn Animation>>,
}

impl SequenceAnimation {
    pub fn new(animations: Vec<Box<dyn Animation>>) -> Self {
        SequenceAnimation { animations }
    }
}

impl Animation for SequenceAnimation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        let last_animation = if delta.is_sign_negative() {
            self.animations.iter_mut().rev().find(|a| a.has_started())
        } else {
            self.animations
                .iter_mut()
                .filter(|a| !a.is_finished())
                .next()
        };
        if let Some(current) = last_animation {
            current.update(delta)
        } else {
            vec![]
        }
    }

    fn is_finished(&self) -> bool {
        self.animations.is_empty() || self.animations.iter().all(|a| a.is_finished())
    }

    fn reset(&mut self) {
        for anim in &mut self.animations {
            anim.reset();
        }
    }
    fn has_started(&self) -> bool {
        self.animations.iter().any(|a| a.has_started())
    }
}

pub struct ParallelAnimation {
    animations: Vec<Box<dyn Animation>>,
}

impl ParallelAnimation {
    pub fn new(animations: Vec<Box<dyn Animation>>) -> Self {
        ParallelAnimation { animations }
    }
}

impl Animation for ParallelAnimation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        let updates = self
            .animations
            .iter_mut()
            .filter(|a| !a.is_finished())
            .flat_map(|anim| anim.update(delta))
            .collect();
        updates
    }

    fn is_finished(&self) -> bool {
        self.animations.iter().all(|anim| anim.is_finished())
    }

    fn reset(&mut self) {
        for anim in &mut self.animations {
            anim.reset();
        }
    }
    fn has_started(&self) -> bool {
        self.animations.iter().any(|anim| anim.has_started())
    }
}

pub struct DelayAnimation {
    delay: f32,
    elapsed: f32,
    animation: Box<dyn Animation>,
}

impl DelayAnimation {
    pub fn new(delay: f32, animation: impl Animation + 'static) -> Self {
        DelayAnimation {
            delay,
            elapsed: 0.0,
            animation: Box::new(animation),
        }
    }
}

impl Animation for DelayAnimation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        if self.elapsed < self.delay {
            self.elapsed += delta;
            if self.elapsed <= self.delay {
                return vec![];
            }
            // Compensa o tempo que passou além do delay.
            let adjusted_delta = self.elapsed - self.delay;
            self.elapsed = self.delay;
            return self.animation.update(adjusted_delta);
        }
        self.animation.update(delta)
    }

    fn is_finished(&self) -> bool {
        self.animation.is_finished()
    }

    fn reset(&mut self) {
        self.elapsed = 0.0;
        self.animation.reset();
    }
    fn has_started(&self) -> bool {
        self.elapsed > 0.0 || self.animation.has_started()
    }
}
pub struct LoopingAnimation {
    animation: Box<dyn Animation>,
    repeat_count: usize,
    current_count: usize,
}

impl LoopingAnimation {
    pub fn new(animation: impl Animation + 'static, repeat_count: usize) -> Self {
        LoopingAnimation {
            animation: Box::new(animation),
            repeat_count,
            current_count: 0,
        }
    }
    pub fn infinite(animation: impl Animation + 'static) -> Self {
        LoopingAnimation::new(animation, 0)
    }
}

impl Animation for LoopingAnimation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        if delta.is_sign_negative() {
            panic!("Looping animation doesn't support playing backwards")
        }
        if self.is_finished() {
            return vec![];
        }
        let updates = self.animation.update(delta);
        if self.animation.is_finished() {
            self.current_count += 1;
            self.animation.reset();
        }
        updates
    }

    fn is_finished(&self) -> bool {
        self.repeat_count != 0 && self.current_count >= self.repeat_count
    }

    fn reset(&mut self) {
        self.current_count = 0;
        self.animation.reset();
    }
    fn has_started(&self) -> bool {
        self.current_count > 0 || self.animation.has_started()
    }
}

pub struct BackAndForthAnimation {
    animation: Box<dyn Animation>,
    repeat_count: usize,
    current_count: usize,
    forward: bool,
}

impl BackAndForthAnimation {
    pub fn new(animation: impl Animation + 'static, repeat_count: usize) -> Self {
        BackAndForthAnimation {
            animation: Box::new(animation),
            repeat_count,
            current_count: 0,
            forward: true,
        }
    }

    pub fn infinite(animation: impl Animation + 'static) -> Self {
        Self::new(animation, 0)
    }
}

impl Animation for BackAndForthAnimation {
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        if delta.is_sign_negative() {
            panic!("Can't run BackAndForthAnimation backwards");
        }
        if self.is_finished() {
            return vec![];
        }
        let updates = self
            .animation
            .update(if self.forward { delta } else { -delta });
        if self.animation.is_finished() || !self.animation.has_started() {
            self.current_count += 1;
            self.forward = !self.forward;
        }
        updates
    }

    fn is_finished(&self) -> bool {
        self.repeat_count != 0 && self.current_count >= self.repeat_count
    }

    fn reset(&mut self) {
        self.current_count = 0;
        self.forward = true;
        self.animation.reset();
    }
    fn has_started(&self) -> bool {
        self.current_count > 0 || self.animation.has_started()
    }
}

pub struct ProgressBarAnimation {
    id: String,
    current: f32,
    target: f32,
    speed: f32,
    receiver: Receiver<f32>, // Channel to receive new target values
}

impl ProgressBarAnimation {
    /// Creates a new ProgressBarAnimation and returns a tuple (ProgressBarAnimation, Sender<f32>).
    /// The `Sender<f32>` allows modifying the target value externally.
    pub fn new(id: &str, speed: f32) -> (Self, Sender<f32>) {
        let (sender, receiver) = mpsc::channel();
        let animation = Self {
            id: id.to_string(),
            current: 0.0,
            target: 0.0,
            speed,
            receiver,
        };
        (animation, sender)
    }
}

impl Animation for ProgressBarAnimation {
    /// Updates progress, moving `current` toward `target` based on the time delta.
    /// If there is a new value in the channel, updates `target`.
    fn update(&mut self, delta: f32) -> Vec<(String, f32)> {
        // Checks if there’s a new target value in the channel and updates if available
        while let Ok(new_target) = self.receiver.try_recv() {
            self.target = new_target.clamp(0.0, 1.0);
        }

        // Moves current smoothly toward target
        // and slow down as it approaches the target (ease out)
        let diff = self.target - self.current;
        let smoothing = 1.0 - (-self.speed * delta).exp();
        self.current += diff * smoothing;
        if self.current >= 0.995 {
            self.current = 1.0
        }
        vec![(self.id.clone(), self.current)]
    }

    /// Considers the animation finished if the current progress is close to the end
    fn is_finished(&self) -> bool {
        self.current >= 0.995 && self.target == 1.0
    }

    /// Returns `true` if the progress has started (i.e., if current > 0).
    fn has_started(&self) -> bool {
        self.current > 0.0
    }

    /// Resets progress to 0 and sets the target to 0.
    fn reset(&mut self) {
        self.current = 0.0;
        self.target = 0.0;
    }
}

// Some common easings for animation
// I have no idea how any of this math works, but it works
pub mod easing {
    pub use std::convert::identity as linear;

    pub fn ease_in_quad(t: f32) -> f32 {
        t * t
    }

    pub fn ease_out_quad(t: f32) -> f32 {
        t * (2.0 - t)
    }

    pub fn ease_in_out_quad(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }

    pub fn ease_in_cubic(t: f32) -> f32 {
        t.powi(3)
    }

    pub fn ease_out_cubic(t: f32) -> f32 {
        let t1 = t - 1.0;
        t1.powi(3) + 1.0
    }

    pub fn ease_in_out_cubic(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t1 = 2.0 * t - 2.0;
            0.5 * t1.powi(3) + 1.0
        }
    }

    pub fn ease_in_quart(t: f32) -> f32 {
        t.powi(4)
    }

    pub fn ease_out_quart(t: f32) -> f32 {
        let t1 = t - 1.0;
        1.0 - t1.powi(4)
    }

    pub fn ease_in_out_quart(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            let t1 = t - 1.0;
            -8.0 * t1 * t1 * t1 * t1 + 1.0
        }
    }

    pub fn ease_in_quint(t: f32) -> f32 {
        t.powi(5)
    }

    pub fn ease_out_quint(t: f32) -> f32 {
        let t1 = t - 1.0;
        t1.powi(5) + 1.0
    }

    pub fn ease_in_out_quint(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            let t1 = 2.0 * t - 2.0;
            0.5 * t1 * t1 * t1 * t1 * t1 + 1.0
        }
    }
    pub fn ease_in_elastic(t: f32) -> f32 {
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            -((2.0_f32).powf(10.0 * t - 10.0)) * (((t * 10.0 - 10.75) * c4).sin())
        }
    }

    pub fn ease_out_elastic(t: f32) -> f32 {
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else {
            ((2.0_f32).powf(-10.0 * t)) * (((t * 10.0 - 0.75) * c4).sin()) + 1.0
        }
    }

    pub fn ease_in_out_elastic(t: f32) -> f32 {
        let c5 = (2.0 * std::f32::consts::PI) / 4.5;
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            -0.5 * ((2.0_f32).powf(20.0 * t - 10.0)) * (((20.0 * t - 11.125) * c5).sin())
        } else {
            ((2.0_f32).powf(-20.0 * t + 10.0)) * (((20.0 * t - 11.125) * c5).sin()) * 0.5 + 1.0
        }
    }
    pub fn ease_out_back(t: f32) -> f32 {
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
    }
    pub fn ease_in_out_circ(t: f32) -> f32 {
        if t < 0.5 {
            (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
        } else {
            ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
        }
    }
}

pub mod colors {
    pub fn interpolate_color(start: (u8, u8, u8), end: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
        let r = start.0 as f32 + t * (end.0 as f32 - start.0 as f32);
        let g = start.1 as f32 + t * (end.1 as f32 - start.1 as f32);
        let b = start.2 as f32 + t * (end.2 as f32 - start.2 as f32);
        (r as u8, g as u8, b as u8)
    }
    pub fn interpolate_color_normalized(
        start: (f32, f32, f32),
        end: (f32, f32, f32),
        t: f32,
    ) -> (f32, f32, f32) {
        let r = start.0 + t * (end.0 - start.0);
        let g = start.1 + t * (end.1 - start.1);
        let b = start.2 + t * (end.2 - start.2);
        (r, g, b)
    }
    pub fn rgb_to_norm(rgb: &str) -> (f32, f32, f32) {
        assert!(rgb.len() == 7 && rgb.starts_with('#'), "Invalid RGB format");
        let r = u8::from_str_radix(&rgb[1..3], 16).expect("Invalid red component") as f32 / 255.0;
        let g = u8::from_str_radix(&rgb[3..5], 16).expect("Invalid green component") as f32 / 255.0;
        let b = u8::from_str_radix(&rgb[5..7], 16).expect("Invalid blue component") as f32 / 255.0;
        (r, g, b)
    }
    pub fn rgba_to_norm(rgb: &str) -> (f32, f32, f32, f32) {
        assert!(rgb.len() == 9 && rgb.starts_with('#'), "Invalid RGB format");
        let r = u8::from_str_radix(&rgb[1..3], 16).expect("Invalid red component") as f32 / 255.0;
        let g = u8::from_str_radix(&rgb[3..5], 16).expect("Invalid green component") as f32 / 255.0;
        let b = u8::from_str_radix(&rgb[5..7], 16).expect("Invalid blue component") as f32 / 255.0;
        let a = u8::from_str_radix(&rgb[7..9], 16).expect("Invalid alpha component") as f32 / 255.0;
        (r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::easing::*;
    use super::*;

    #[test]
    fn test_basic_animation() {
        let mut anim = BasicAnimation::new("test", 1.0, linear);
        assert_eq!(anim.update(0.5), vec![("test".to_string(), 0.5)]);
        assert_eq!(anim.update(0.5), vec![("test".to_string(), 1.0)]);
        assert!(anim.is_finished());
        anim.reset();
        assert_eq!(anim.update(0.5), vec![("test".to_string(), 0.5)]);
    }

    #[test]
    fn test_sequence_animation() {
        let anim1 = BasicAnimation::new("anim1", 1.0, linear);
        let anim2 = BasicAnimation::new("anim2", 1.0, linear);
        let mut seq = SequenceAnimation::new(vec![Box::new(anim1), Box::new(anim2)]);
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 0.5)]);
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 1.0)]);
        assert_eq!(seq.update(0.5), vec![("anim2".to_string(), 0.5)]);
        assert_eq!(seq.update(0.5), vec![("anim2".to_string(), 1.0)]);
        assert!(seq.is_finished());
        seq.reset();
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 0.5)]);
    }

    #[test]
    fn test_parallel_animation() {
        let anim1 = BasicAnimation::new("anim1", 1.0, linear);
        let anim2 = BasicAnimation::new("anim2", 2.0, linear);
        let mut par = ParallelAnimation::new(vec![Box::new(anim1), Box::new(anim2)]);
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 0.5), ("anim2".to_string(), 0.25)]
        );
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 1.0), ("anim2".to_string(), 0.5)]
        );
        assert_eq!(par.update(1.0), vec![("anim2".to_string(), 1.0)]);
        assert!(par.is_finished());
        par.reset();
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 0.5), ("anim2".to_string(), 0.25)]
        );
    }

    #[test]
    fn test_delay_animation() {
        let anim = BasicAnimation::new("anim", 1.0, linear);
        let mut delay = DelayAnimation::new(1.0, anim);
        assert_eq!(delay.update(0.5), vec![]);
        assert_eq!(delay.update(0.5), vec![]);
        assert_eq!(delay.update(0.5), vec![("anim".to_string(), 0.5)]);
        assert_eq!(delay.update(0.5), vec![("anim".to_string(), 1.0)]);
        assert!(delay.is_finished());
        delay.reset();
        assert_eq!(delay.update(0.5), vec![]);
    }

    #[test]
    fn test_easing_functions() {
        assert_eq!(ease_in_quad(0.5), 0.25);
        assert_eq!(ease_out_quad(0.5), 0.75);
        assert_eq!(ease_in_out_quad(0.5), 0.5);
        assert_eq!(ease_in_cubic(0.5), 0.125);
        assert_eq!(ease_out_cubic(0.5), 0.875);
        assert_eq!(ease_in_out_cubic(0.5), 0.5);
        assert_eq!(ease_in_quart(0.5), 0.0625);
        assert_eq!(ease_out_quart(0.5), 0.9375);
        assert_eq!(ease_in_out_quart(0.5), 0.5);
        assert_eq!(ease_in_quint(0.5), 0.03125);
        assert_eq!(ease_out_quint(0.5), 0.96875);
        assert_eq!(ease_in_out_quint(0.5), 0.5);
    }
}
#[macro_export]
macro_rules! all {
	() => {$crate::animation::ParallelAnimation::new(vec![])};
    ( $( $x:expr ),* ) => {
        {
            let mut animations: Vec<Box<dyn $crate::animation::Animation>> = Vec::new();
            $(
                animations.push(Box::new($x));
            )*
            $crate::animation::ParallelAnimation::new(animations)
        }
    };
}

#[macro_export]
macro_rules! seq {
    ( $( $x:expr ),* ) => {
        {
            let mut animations: Vec<Box<dyn $crate::animation::Animation>> = Vec::new();
            $(
                animations.push(Box::new($x));
            )*
            $crate::animation::SequenceAnimation::new(animations)
        }
    };
}

#[cfg(test)]
mod macro_tests {
    use super::easing::*;
    use super::*;

    #[test]
    fn test_all_macro() {
        let anim1 = BasicAnimation::new("anim1", 1.0, linear);
        let anim2 = BasicAnimation::new("anim2", 2.0, linear);
        let mut par = all!(anim1, anim2);
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 0.5), ("anim2".to_string(), 0.25)]
        );
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 1.0), ("anim2".to_string(), 0.5)]
        );
        assert_eq!(par.update(1.0), vec![("anim2".to_string(), 1.0)]);
        assert!(par.is_finished());
        par.reset();
        assert_eq!(
            par.update(0.5),
            vec![("anim1".to_string(), 0.5), ("anim2".to_string(), 0.25)]
        );
    }

    #[test]
    fn test_seq_macro() {
        let anim1 = BasicAnimation::new("anim1", 1.0, linear);
        let anim2 = BasicAnimation::new("anim2", 1.0, linear);
        let mut seq = seq!(anim1, anim2);
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 0.5)]);
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 1.0)]);
        assert_eq!(seq.update(0.5), vec![("anim2".to_string(), 0.5)]);
        assert_eq!(seq.update(0.5), vec![("anim2".to_string(), 1.0)]);
        assert!(seq.is_finished());
        seq.reset();
        assert_eq!(seq.update(0.5), vec![("anim1".to_string(), 0.5)]);
    }
}
