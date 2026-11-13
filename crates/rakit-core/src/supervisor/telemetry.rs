use crate::models::ServiceInstance;
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, System};
use tokio::sync::RwLock;

pub struct TelemetryCollector;

impl TelemetryCollector {
    pub async fn poll_metrics(instances: Arc<RwLock<HashMap<String, ServiceInstance>>>) {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut map = instances.write().await;
        for instance in map.values_mut() {
            if let Some(pid_val) = instance.pid {
                let pid = Pid::from_u32(pid_val);
                if let Some(process) = sys.process(pid) {
                    instance.cpu_percent = process.cpu_usage();
                    instance.memory_bytes = process.memory();
                    instance.uptime_secs = process.run_time();
                } else {
                    instance.memory_bytes = 0;
                    instance.cpu_percent = 0.0;
                }
            }
        }
    }
}
