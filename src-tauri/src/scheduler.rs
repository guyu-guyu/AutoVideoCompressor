use chrono::{DateTime, Duration, Local, TimeZone};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// One directory's schedule entry. Mirrors DirSchedule (C++).
#[derive(Clone)]
pub struct DirSchedule {
    pub dir_path: String,
    pub enabled: bool,
    pub next_run: DateTime<Local>,
}

/// Per-directory timetable poller. Mirrors Scheduler (C++).
pub struct Scheduler {
    schedules: Arc<Mutex<Vec<DirSchedule>>>,
    running: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Pure: next run for HH:MM relative to `now`. Mirrors computeNextRun.
pub fn compute_next_run(now: DateTime<Local>, hour: u32, minute: u32) -> DateTime<Local> {
    let today = now.date_naive().and_hms_opt(hour, minute, 0).unwrap();
    let mut target = Local.from_local_datetime(&today).single().unwrap_or(now);
    if target <= now {
        target += Duration::hours(24);
    }
    target
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            schedules: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
        }
    }

    pub fn set_directories(&self, dirs: Vec<DirSchedule>) {
        *self.schedules.lock().unwrap() = dirs;
    }

    pub fn seconds_until_next_run(&self, dir_path: &str) -> i64 {
        let guard = self.schedules.lock().unwrap();
        for s in guard.iter() {
            if s.dir_path == dir_path {
                if !s.enabled { return -1; }
                let diff = (s.next_run - Local::now()).num_seconds();
                return diff.max(0);
            }
        }
        -1
    }

    pub fn next_run_time(&self, dir_path: &str) -> Option<DateTime<Local>> {
        let guard = self.schedules.lock().unwrap();
        for s in guard.iter() {
            if s.dir_path == dir_path {
                if !s.enabled { return None; }
                return Some(s.next_run);
            }
        }
        None
    }

    pub fn mark_completed(&self, dir_path: &str) {
        let mut guard = self.schedules.lock().unwrap();
        for s in guard.iter_mut() {
            if s.dir_path == dir_path {
                s.next_run += Duration::hours(24);
                break;
            }
        }
    }

    /// Start the background poller. `cb` is called with the directory to run.
    pub fn start<F>(&self, cb: F)
    where
        F: Fn(String) + Send + 'static,
    {
        self.running.store(true, Ordering::SeqCst);
        let schedules = Arc::clone(&self.schedules);
        let running = Arc::clone(&self.running);
        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let mut to_run: Option<String> = None;
                {
                    let now = Local::now();
                    let guard = schedules.lock().unwrap();
                    for s in guard.iter() {
                        if s.enabled && now >= s.next_run {
                            to_run = Some(s.dir_path.clone());
                            break;
                        }
                    }
                }
                if let Some(dir) = to_run {
                    cb(dir);
                }
                thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        *self.handle.lock().unwrap() = Some(handle);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self { Scheduler::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Duration, Timelike};

    #[test]
    fn compute_next_run_future_today() {
        let now = Local::now();
        // pick a minute one hour ahead → still today
        let target = now + Duration::hours(1);
        let next = compute_next_run(now, target.hour(), target.minute());
        assert!(next > now);
        // within ~1 day
        assert!(next <= now + Duration::hours(25));
    }

    #[test]
    fn compute_next_run_past_rolls_tomorrow() {
        let now = Local::now();
        let past = now - Duration::hours(1);
        let next = compute_next_run(now, past.hour(), past.minute());
        assert!(next > now);
    }

    #[test]
    fn seconds_and_mark_completed() {
        let sched = Scheduler::new();
        let now = Local::now();
        let future = now + Duration::hours(2);
        sched.set_directories(vec![DirSchedule {
            dir_path: "D:/x".into(),
            enabled: true,
            next_run: future,
        }]);
        let secs = sched.seconds_until_next_run("D:/x");
        assert!(secs > 0);
        assert_eq!(sched.seconds_until_next_run("D:/missing"), -1);
        sched.mark_completed("D:/x");
        let after = sched.next_run_time("D:/x").unwrap();
        assert!(after > future);
    }
}
