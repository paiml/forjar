//! Demonstrates FJ-2703/2704 GPU scheduling and barrier tasks.

use forjar::core::types::{BarrierTask, GpuSchedule};

fn main() {
    // Multi-GPU round-robin scheduling
    println!("=== GPU Schedule (Round-Robin) ===");
    let schedule = GpuSchedule::round_robin(&["train-a", "train-b", "train-c", "train-d"], 2);
    for (task, devs) in &schedule.assignments {
        println!(
            "  {task}: CUDA_VISIBLE_DEVICES={}",
            schedule.cuda_visible_devices(task).unwrap()
        );
        println!("    devices: {devs:?}");
    }
    println!("  Total devices: {}", schedule.total_devices);
    println!("  Assigned: {}", schedule.assigned_device_count());
    println!("  Fully utilized: {}", schedule.fully_utilized());

    // Manual assignment (multi-GPU model)
    println!("\n=== GPU Schedule (Manual) ===");
    let mut manual = GpuSchedule::new(4);
    manual.assign("large-model", vec![0, 1, 2, 3]);
    manual.assign("small-eval", vec![0]);
    println!(
        "  large-model: CUDA_VISIBLE_DEVICES={}",
        manual.cuda_visible_devices("large-model").unwrap()
    );
    println!(
        "  small-eval: CUDA_VISIBLE_DEVICES={}",
        manual.cuda_visible_devices("small-eval").unwrap()
    );
    println!("  Fully utilized: {}", manual.fully_utilized());

    // Barrier task for multi-machine synchronization
    println!("\n=== Barrier Task ===");
    let mut barrier = BarrierTask::new("sync-training", vec![
        "gpu-0".into(),
        "gpu-1".into(),
        "gpu-2".into(),
    ]);
    barrier.timeout_secs = 300;
    println!("  {barrier}");

    barrier.mark_complete("gpu-0");
    println!("  After gpu-0: {barrier}");
    println!("    Progress: {:.0}%", barrier.progress_pct());
    println!("    Pending: {:?}", barrier.pending_machines());

    barrier.mark_complete("gpu-1");
    barrier.mark_complete("gpu-2");
    println!("  After all: {barrier}");
    println!("    Satisfied: {}", barrier.is_satisfied());
}
