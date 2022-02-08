use quanta::Clock;
use serde::ser::SerializeMap;
use serde::{Serialize, Deserialize, Serializer};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

pub type FuncTimer = HashMap<String, TimerState>;

#[derive(Clone)]
pub struct FuncTimerDisplay(FuncTimer);

impl FuncTimerDisplay {
    pub fn new(t: &FuncTimer) -> Self {
        FuncTimerDisplay(t.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerDisplayContainer {
    pub key: String,
    pub nanos: u128,
    pub ms: u128,
}

#[derive(Debug, Clone)]
pub struct TimerState {
    pub current_clock: Clock,
    pub start_time: u64,
    pub end_time: u64,
}

impl TimerState {
    pub fn new() -> Self {
        let clock = Clock::new();
        let start = clock.start();

        TimerState {
            current_clock: clock,
            start_time: start,
            end_time: start,
        }
    }

    pub fn start_time(&mut self) {
        self.start_time = self.current_clock.start();
    }

    pub fn stop_time(&mut self) {
        self.end_time = self.current_clock.end();
    }

    pub fn elapsed(&self) -> Duration {
        self.current_clock.delta(self.start_time, self.end_time)
    }
}

pub trait Timer {
    fn start(&mut self, _: String) {
        return;
    }

    fn stop(&mut self, _: String) {
        return;
    }

    fn elapsed(&self, _: String) -> Option<Duration> {
        return None;
    }
}

impl Timer for HashMap<String, TimerState> {
    #[cfg(feature = "func_timer")]
    fn start(&mut self, name: String) {
        self.entry(name)
            .and_modify(|entry| entry.start_time())
            .or_insert(TimerState::new());
    }

    #[cfg(feature = "func_timer")]
    fn stop(&mut self, name: String) {
        match self.get_mut(&name) {
            None => println!("Warning: timer {} was stopped but does not exist", name),
            Some(entry) => {
                entry.stop_time();
            }
        }
    }

    #[cfg(feature = "func_timer")]
    fn elapsed(&self, name: String) -> Option<Duration> {
        match self.get(&name) {
            None => None,
            Some(ts) => Some(ts.elapsed()),
        }
    }
}

#[cfg(feature = "func_timer")]
impl fmt::Display for TimerState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ns | {} ms",
            self.elapsed().as_nanos(),
            self.elapsed().as_millis()
        )
    }
}

#[cfg(feature = "func_timer")]
impl fmt::Display for FuncTimerDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut sorted_times: Vec<(&String, &TimerState)> = self.0.iter().collect();
        sorted_times.sort_by(|a, b| b.1.start_time.cmp(&a.1.start_time));
        sorted_times.reverse();
        write!(
            f,
            "{}{}{}",
            format!(
                "\n{:?} | {:?} | {:?}\n",
                "Name", "process time", "elapsed time",
            ),
            "----------------------------------------------------------------------\n",
            sorted_times
                .into_iter()
                .map(|(key, value)| {
                    return format!("{:?} | {}\n", key, format!("{}", value));
                })
                .collect::<Vec<String>>()
                .join("")
        )
    }
}

#[cfg(feature = "func_timer")]
pub fn get_sorted_times(display_timer: &FuncTimerDisplay) -> Vec<TimerDisplayContainer> {
    let mut sorted_times: Vec<(&String, &TimerState)> = display_timer.0.iter().collect();
    sorted_times.sort_by(|a, b| b.1.start_time.cmp(&a.1.start_time));
    sorted_times.reverse();
    return sorted_times
            .into_iter()
            .map(|(key, value)| {
                return TimerDisplayContainer {key: key.to_string(), nanos: value.elapsed().as_nanos(), ms: value.elapsed().as_millis()};
            })
            .collect::<Vec<TimerDisplayContainer>>();
}

#[cfg(not(feature = "func_timer"))]
impl fmt::Display for FuncTimerDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            ""
        )
    }
}

//The above code is a copy of Breezy Timer crate with modifications

// MIT License

// Copyright (c) 2021 Edoardo Barp

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
