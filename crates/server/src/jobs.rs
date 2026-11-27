//! Deploy jobs, kept in memory only.
//!
//! A job holds no API token: the provisioning task owns the token and drops it when it ends. The
//! generated admin password lives here so the wizard can show it once, and the whole job is
//! forgotten an hour after it was created (or on restart).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use types::JobStatus;

pub const JOB_TTL: Duration = Duration::from_secs(60 * 60);
/// A cap so a flood of requests cannot grow memory without bound.
pub const MAX_JOBS: usize = 500;

#[derive(Clone, Default)]
pub struct Jobs {
    inner: Arc<Mutex<HashMap<String, Job>>>,
}

#[derive(Clone)]
pub struct Job {
    pub created: Instant,
    pub status: JobStatus,
}

impl Jobs {
    /// Registers a new job and returns its unguessable id, or `None` when the table is full.
    pub fn create(&self) -> Option<String> {
        let mut map = self.inner.lock().unwrap();
        map.retain(|_, j| j.created.elapsed() < JOB_TTL);
        if map.len() >= MAX_JOBS {
            return None;
        }
        let id = random_id();
        map.insert(
            id.clone(),
            Job {
                created: Instant::now(),
                status: JobStatus {
                    job_id: id.clone(),
                    status: "queued".into(),
                    detail: None,
                    ip: None,
                    url: None,
                    admin_user: None,
                    admin_password: None,
                },
            },
        );
        Some(id)
    }

    pub fn update(&self, id: &str, f: impl FnOnce(&mut JobStatus)) {
        if let Some(job) = self.inner.lock().unwrap().get_mut(id) {
            f(&mut job.status);
        }
    }

    pub fn get(&self, id: &str) -> Option<JobStatus> {
        let map = self.inner.lock().unwrap();
        map.get(id)
            .filter(|j| j.created.elapsed() < JOB_TTL)
            .map(|j| j.status.clone())
    }
}

/// 128 random bits, hex. Job ids double as the only access check for a job's status.
fn random_id() -> String {
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).expect("the OS random number generator works");
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_random_and_jobs_update() {
        let jobs = Jobs::default();
        let a = jobs.create().unwrap();
        let b = jobs.create().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        jobs.update(&a, |s| s.status = "done".into());
        assert_eq!(jobs.get(&a).unwrap().status, "done");
        assert_eq!(jobs.get(&b).unwrap().status, "queued");
        assert!(jobs.get("nope").is_none());
    }
}
