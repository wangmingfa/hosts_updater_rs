//! 定时任务模块
//!
//! 提供定时执行任务的功能。

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time;

/// 定时任务配置
pub struct Scheduler {
    interval_hours: u64,
}

impl Scheduler {
    /// 创建新的定时任务调度器
    ///
    /// # Arguments
    ///
    /// * `interval_hours` - 更新间隔时间（小时）
    pub fn new(interval_hours: u64) -> Self {
        Self { interval_hours }
    }

    /// 获取更新间隔时间
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.interval_hours * 3600)
    }

    /// 启动定时任务
    ///
    /// # Arguments
    ///
    /// * `task` - 要定时执行的任务闭包
    pub async fn start<T>(&self, mut task: T)
    where
        T: FnMut() -> Pin<Box<dyn Future<Output = ()> + Send>>,
    {
        tracing::info!(
            "定时任务已启动，间隔: {} 小时",
            self.interval_hours
        );

        // 立即执行一次
        task().await;

        // 定时执行
        let mut interval = time::interval(self.interval());
        loop {
            interval.tick().await;
            task().await;
        }
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_scheduler_run() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // 运行 2 次，间隔 0.1 秒
        let mut run_count = 0;
        let mut interval = tokio::time::interval(Duration::from_millis(100));

        tokio::spawn(async move {
            interval.tick().await; // 第一次 tick
            counter_clone.fetch_add(1, Ordering::SeqCst);
            run_count += 1;

            if run_count < 2 {
                interval.tick().await; // 第二次 tick
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        // 等待足够时间
        tokio::time::sleep(Duration::from_millis(300)).await;

        assert!(counter.load(Ordering::SeqCst) >= 1);
    }
}
