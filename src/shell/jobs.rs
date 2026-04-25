// cash — Job Control
//
// Background jobs: cmd &
// Commands: jobs, fg, bg
//
// Unix only — uses process groups and signals.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum JobState {
    Running,
    Stopped,
    Done(i32),
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id:      usize,
    pub pid:     u32,
    pub command: String,
    pub state:   JobState,
}

impl Job {
    pub fn state_str(&self) -> &str {
        match self.state {
            JobState::Running   => "Running",
            JobState::Stopped   => "Stopped",
            JobState::Done(_)   => "Done",
        }
    }
}

pub struct JobTable {
    jobs:    Arc<Mutex<HashMap<usize, Job>>>,
    next_id: Arc<Mutex<usize>>,
}

impl JobTable {
    pub fn new() -> Self {
        Self {
            jobs:    Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Register a new background job.
    pub fn add(&self, pid: u32, command: &str) -> usize {
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        self.jobs.lock().unwrap().insert(id, Job {
            id,
            pid,
            command: command.to_string(),
            state: JobState::Running,
        });
        id
    }

    /// List all jobs.
    pub fn list(&self) -> Vec<Job> {
        let mut jobs: Vec<Job> = self.jobs.lock().unwrap().values().cloned().collect();
        jobs.sort_by_key(|j| j.id);
        jobs
    }

    /// Print jobs table.
    pub fn print(&self) {
        let jobs = self.list();
        if jobs.is_empty() {
            println!("No background jobs.");
            return;
        }
        for job in &jobs {
            println!("[{}]  {:8}  {}", job.id, job.state_str(), job.command);
        }
    }

    /// Mark a job as done.
    pub fn mark_done(&self, id: usize, exit_code: i32) {
        if let Some(job) = self.jobs.lock().unwrap().get_mut(&id) {
            job.state = JobState::Done(exit_code);
        }
    }

    /// Mark a job as stopped.
    pub fn mark_stopped(&self, id: usize) {
        if let Some(job) = self.jobs.lock().unwrap().get_mut(&id) {
            job.state = JobState::Stopped;
        }
    }

    /// Remove completed jobs.
    pub fn clean(&self) {
        self.jobs.lock().unwrap().retain(|_, j| j.state != JobState::Running);
    }

    /// Get job by ID.
    pub fn get(&self, id: usize) -> Option<Job> {
        self.jobs.lock().unwrap().get(&id).cloned()
    }

    /// Get most recent job.
    pub fn last(&self) -> Option<Job> {
        self.jobs.lock().unwrap().values()
            .max_by_key(|j| j.id)
            .cloned()
    }

    pub fn count(&self) -> usize {
        self.jobs.lock().unwrap().len()
    }

    /// Send SIGCONT to a stopped job (fg/bg).
    #[cfg(unix)]
    pub fn continue_job(&self, id: usize) -> Result<(), String> {
        let job = self.get(id).ok_or_else(|| format!("job {} not found", id))?;
        unsafe {
            if libc_kill(job.pid as i32, 18) != 0 { // SIGCONT = 18
                return Err(format!("failed to continue job {}", id));
            }
        }
        if let Some(j) = self.jobs.lock().unwrap().get_mut(&id) {
            j.state = JobState::Running;
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn continue_job(&self, _id: usize) -> Result<(), String> {
        Err("job control not available on this platform".into())
    }
}

#[cfg(unix)]
extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
unsafe fn libc_kill(pid: i32, sig: i32) -> i32 {
    kill(pid, sig)
}

impl Default for JobTable {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_job() {
        let table = JobTable::new();
        let id = table.add(1234, "sleep 100");
        assert_eq!(table.count(), 1);
        let jobs = table.list();
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].command, "sleep 100");
        assert_eq!(jobs[0].state, JobState::Running);
    }

    #[test]
    fn mark_done() {
        let table = JobTable::new();
        let id = table.add(1234, "sleep 100");
        table.mark_done(id, 0);
        let job = table.get(id).unwrap();
        assert_eq!(job.state, JobState::Done(0));
    }

    #[test]
    fn last_job() {
        let table = JobTable::new();
        table.add(1234, "sleep 10");
        table.add(5678, "sleep 20");
        let last = table.last().unwrap();
        assert_eq!(last.command, "sleep 20");
    }

    #[test]
    fn job_ids_increment() {
        let table = JobTable::new();
        let id1 = table.add(1, "cmd1");
        let id2 = table.add(2, "cmd2");
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn print_empty_jobs() {
        let table = JobTable::new();
        table.print(); // should not panic
    }
}
