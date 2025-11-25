// Test starting a Cloud Hypervisor VM without network
use std::process::Command;
use std::path::PathBuf;

fn main() {
    println!("=== Testing Cloud Hypervisor VM (No Network) ===\n");

    // Check if we have the necessary files
    let kernel_path = "/nix/store/ydaj8kmad8kq4c6jgc1m1a7z3yx58w62-linux-6.12.58-dev/vmlinux";
    let initrd_path = "/nix/store/a12ngmw7lhda7yknbs8i9j70gvn01zwl-initrd-linux-6.12.58/initrd";
    let ch_path = "/nix/store/p77vhx0vliy7n9i2bjyfz9xa4a321ihw-cloud-hypervisor-49.0/bin/cloud-hypervisor";

    // Check files exist
    if !PathBuf::from(kernel_path).exists() {
        println!("❌ Kernel not found at: {}", kernel_path);
        println!("   Run: nix build ./microvms#nixosConfigurations.worker-vm.microvm.runner");
        return;
    }
    if !PathBuf::from(initrd_path).exists() {
        println!("❌ Initrd not found at: {}", initrd_path);
        return;
    }
    if !PathBuf::from(ch_path).exists() {
        println!("❌ Cloud Hypervisor not found at: {}", ch_path);
        return;
    }

    println!("✓ All required files found");
    println!("Starting Cloud Hypervisor VM without networking...\n");

    // Start Cloud Hypervisor with minimal config (no network)
    let mut cmd = Command::new(ch_path);
    cmd.args(&[
        "--cpus", "boot=1",
        "--memory", "size=512M",
        "--kernel", kernel_path,
        "--initramfs", initrd_path,
        "--cmdline", "console=hvc0 root=/dev/vda init=/init",
        "--console", "off",
        "--serial", "tty",
        "--api-socket", "/tmp/ch-test.sock"
    ]);

    println!("Command: {:?}\n", cmd);

    // Start the VM
    match cmd.spawn() {
        Ok(mut child) => {
            println!("✓ VM process started with PID: {}", child.id());

            // Wait a bit to see if it starts properly
            println!("Waiting 3 seconds...");
            std::thread::sleep(std::time::Duration::from_secs(3));

            // Check if process is still running
            match child.try_wait() {
                Ok(None) => {
                    println!("✓ VM is running!");

                    // Kill the VM
                    println!("Stopping VM...");
                    let _ = child.kill();
                    let _ = child.wait();
                    println!("✓ VM stopped");
                }
                Ok(Some(status)) => {
                    println!("❌ VM exited with status: {}", status);
                    if !status.success() {
                        // Try to get more info
                        println!("\nTroubleshooting:");
                        println!("1. This might need to run as root");
                        println!("2. Check dmesg for KVM errors");
                        println!("3. Ensure KVM module is loaded: lsmod | grep kvm");
                    }
                }
                Err(e) => println!("❌ Failed to check VM status: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Failed to start VM: {}", e);
            println!("\nCommon issues:");
            println!("1. Permission denied - may need root or kvm group membership");
            println!("2. KVM not available - check /dev/kvm exists");
            println!("3. CPU virtualization not enabled in BIOS");
        }
    }

    println!("\nTest complete!");
}