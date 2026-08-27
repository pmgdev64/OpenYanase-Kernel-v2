// src/bugcheck.rs

use core::fmt::Write;

// ==========================================
// BUGCHECK CODES (from BugCheck Codes table)
// ==========================================

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BugCheckCode {
    // HIGH severity
    MemoryPressure = 0x00000001,
    MemoryReclaimFailure = 0x00000002,
    KernelHeapCorruption = 0x00000003,
    KernelStackFailure = 0x00000004,
    KernelInvariant = 0x00000005,
    SchedulerFailure = 0x00000006,
    ProcessManagerFailure = 0x00000007,
    InitFailure = 0x00000008,
    IpcFailure = 0x00000009,
    MemoryManagerFailure = 0x0000000A,
    PageTableFailure = 0x0000000B,
    PageFaultFatal = 0x0000000C,
    DoubleFault = 0x0000000D,
    TripleFault = 0x0000000E,
    InterruptFailure = 0x0000000F,
    IdtFailure = 0x00000010,
    GdtFailure = 0x00000011,
    SyscallFailure = 0x00000012,
    VmSupervisorFailure = 0x00000013,
    DriverSupervisor = 0x00000014,
    GpuRecoveryFailure = 0x00000015,
    DriverSandboxFailure = 0x00000016,
    CapabilityEngineFailure = 0x00000017,
    SecurityInvariant = 0x00000018,
    DmaProtectionFailure = 0x00000019,
    HardwareAbstractionFailure = 0x0000001A,
    FilesystemFailure = 0x0000001B,
    RootfsFailure = 0x0000001C,
    KernelTtyFailure = 0x0000001D,
    BootStateFailure = 0x0000001E,
    KernelLogFailure = 0x0000001F,
    KernelRecoveryFailure = 0x00000020,
    KernelStateCorruption = 0x00000021,
    SecurityBoundaryFailure = 0x00000022,
    MemoryIsolationFailure = 0x00000023,
    PrivilegeBoundaryFailure = 0x00000024,
    KernelCodeCorruption = 0x00000025,
    KernelMemoryCorruption = 0x00000026,
    TrustModelFailure = 0x00000027,
    KernelCoreFailure = 0x00000028,
    SystemIntegrityFailure = 0x00000029,
    IsolationFailure = 0x0000002A,
    PanicHandlerFailure = 0x0000002B,
    TotalKernelFailure = 0x0000002C,
    UnknownFatalFailure = 0x0000002D,
    TimerFailure = 0x0000002E,
    ClockFailure = 0x0000002F,
    ContextSwitchFailure = 0x00000030,
    CpuStateFailure = 0x00000031,
    ApicFailure = 0x00000032,
    SmpFailure = 0x00000033,
    DeviceManagerFailure = 0x00000034,
    IoManagerFailure = 0x00000035,
    StorageFailure = 0x00000036,
    BootloaderFailure = 0x00000037,
    SecureBootFailure = 0x00000038,
    CryptoEngineFailure = 0x00000039,
    RngFailure = 0x0000003A,
    AcpiFailure = 0x0000003B,
    ShutdownFailure = 0x0000003C,
}

impl BugCheckCode {
    pub fn severity(&self) -> &'static str {
        match *self {
            BugCheckCode::MemoryPressure
            | BugCheckCode::MemoryReclaimFailure
            | BugCheckCode::KernelHeapCorruption
            | BugCheckCode::KernelStackFailure
            | BugCheckCode::KernelInvariant
            | BugCheckCode::SchedulerFailure
            | BugCheckCode::ProcessManagerFailure
            | BugCheckCode::InitFailure
            | BugCheckCode::IpcFailure
            | BugCheckCode::MemoryManagerFailure
            | BugCheckCode::PageTableFailure
            | BugCheckCode::PageFaultFatal
            | BugCheckCode::DoubleFault
            | BugCheckCode::InterruptFailure
            | BugCheckCode::IdtFailure
            | BugCheckCode::GdtFailure
            | BugCheckCode::SyscallFailure
            | BugCheckCode::DriverSupervisor
            | BugCheckCode::HardwareAbstractionFailure
            | BugCheckCode::FilesystemFailure
            | BugCheckCode::BootStateFailure
            | BugCheckCode::KernelLogFailure
            | BugCheckCode::TimerFailure
            | BugCheckCode::ClockFailure
            | BugCheckCode::ContextSwitchFailure
            | BugCheckCode::CpuStateFailure
            | BugCheckCode::DeviceManagerFailure
            | BugCheckCode::IoManagerFailure
            | BugCheckCode::BootloaderFailure
            | BugCheckCode::AcpiFailure => "HIGH",

            BugCheckCode::TripleFault
            | BugCheckCode::VmSupervisorFailure
            | BugCheckCode::GpuRecoveryFailure
            | BugCheckCode::DriverSandboxFailure
            | BugCheckCode::CapabilityEngineFailure
            | BugCheckCode::SecurityInvariant
            | BugCheckCode::DmaProtectionFailure
            | BugCheckCode::RootfsFailure
            | BugCheckCode::KernelTtyFailure
            | BugCheckCode::KernelRecoveryFailure
            | BugCheckCode::KernelStateCorruption
            | BugCheckCode::SecurityBoundaryFailure
            | BugCheckCode::MemoryIsolationFailure
            | BugCheckCode::PrivilegeBoundaryFailure
            | BugCheckCode::KernelCodeCorruption
            | BugCheckCode::KernelMemoryCorruption
            | BugCheckCode::TrustModelFailure
            | BugCheckCode::ApicFailure
            | BugCheckCode::SmpFailure
            | BugCheckCode::StorageFailure
            | BugCheckCode::SecureBootFailure
            | BugCheckCode::CryptoEngineFailure
            | BugCheckCode::RngFailure
            | BugCheckCode::ShutdownFailure => "CRITICAL",

            BugCheckCode::KernelCoreFailure
            | BugCheckCode::SystemIntegrityFailure
            | BugCheckCode::IsolationFailure
            | BugCheckCode::PanicHandlerFailure
            | BugCheckCode::TotalKernelFailure => "CATASTROPHIC",

            BugCheckCode::UnknownFatalFailure => "FATAL",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match *self {
            BugCheckCode::MemoryPressure => "MEMORY_PRESSURE",
            BugCheckCode::MemoryReclaimFailure => "MEMORY_RECLAIM_FAILURE",
            BugCheckCode::KernelHeapCorruption => "KERNEL_HEAP_CORRUPTION",
            BugCheckCode::KernelStackFailure => "KERNEL_STACK_FAILURE",
            BugCheckCode::KernelInvariant => "KERNEL_INVARIANT",
            BugCheckCode::SchedulerFailure => "SCHEDULER_FAILURE",
            BugCheckCode::ProcessManagerFailure => "PROCESS_MANAGER_FAILURE",
            BugCheckCode::InitFailure => "INIT_FAILURE",
            BugCheckCode::IpcFailure => "IPC_FAILURE",
            BugCheckCode::MemoryManagerFailure => "MEMORY_MANAGER_FAILURE",
            BugCheckCode::PageTableFailure => "PAGE_TABLE_FAILURE",
            BugCheckCode::PageFaultFatal => "PAGE_FAULT_FATAL",
            BugCheckCode::DoubleFault => "DOUBLE_FAULT",
            BugCheckCode::TripleFault => "TRIPLE_FAULT",
            BugCheckCode::InterruptFailure => "INTERRUPT_FAILURE",
            BugCheckCode::IdtFailure => "IDT_FAILURE",
            BugCheckCode::GdtFailure => "GDT_FAILURE",
            BugCheckCode::SyscallFailure => "SYSCALL_FAILURE",
            BugCheckCode::VmSupervisorFailure => "VM_SUPERVISOR_FAILURE",
            BugCheckCode::DriverSupervisor => "DRIVER_SUPERVISOR",
            BugCheckCode::GpuRecoveryFailure => "GPU_RECOVERY_FAILURE",
            BugCheckCode::DriverSandboxFailure => "DRIVER_SANDBOX_FAILURE",
            BugCheckCode::CapabilityEngineFailure => "CAPABILITY_ENGINE_FAILURE",
            BugCheckCode::SecurityInvariant => "SECURITY_INVARIANT",
            BugCheckCode::DmaProtectionFailure => "DMA_PROTECTION_FAILURE",
            BugCheckCode::HardwareAbstractionFailure => "HARDWARE_ABSTRACTION_FAILURE",
            BugCheckCode::FilesystemFailure => "FILESYSTEM_FAILURE",
            BugCheckCode::RootfsFailure => "ROOTFS_FAILURE",
            BugCheckCode::KernelTtyFailure => "KERNELTTY_FAILURE",
            BugCheckCode::BootStateFailure => "BOOT_STATE_FAILURE",
            BugCheckCode::KernelLogFailure => "KERNEL_LOG_FAILURE",
            BugCheckCode::KernelRecoveryFailure => "KERNEL_RECOVERY_FAILURE",
            BugCheckCode::KernelStateCorruption => "KERNEL_STATE_CORRUPTION",
            BugCheckCode::SecurityBoundaryFailure => "SECURITY_BOUNDARY_FAILURE",
            BugCheckCode::MemoryIsolationFailure => "MEMORY_ISOLATION_FAILURE",
            BugCheckCode::PrivilegeBoundaryFailure => "PRIVILEGE_BOUNDARY_FAILURE",
            BugCheckCode::KernelCodeCorruption => "KERNEL_CODE_CORRUPTION",
            BugCheckCode::KernelMemoryCorruption => "KERNEL_MEMORY_CORRUPTION",
            BugCheckCode::TrustModelFailure => "TRUST_MODEL_FAILURE",
            BugCheckCode::KernelCoreFailure => "KERNEL_CORE_FAILURE",
            BugCheckCode::SystemIntegrityFailure => "SYSTEM_INTEGRITY_FAILURE",
            BugCheckCode::IsolationFailure => "ISOLATION_FAILURE",
            BugCheckCode::PanicHandlerFailure => "PANIC_HANDLER_FAILURE",
            BugCheckCode::TotalKernelFailure => "TOTAL_KERNEL_FAILURE",
            BugCheckCode::UnknownFatalFailure => "UNKNOWN_FATAL_FAILURE",
            BugCheckCode::TimerFailure => "TIMER_FAILURE",
            BugCheckCode::ClockFailure => "CLOCK_FAILURE",
            BugCheckCode::ContextSwitchFailure => "CONTEXT_SWITCH_FAILURE",
            BugCheckCode::CpuStateFailure => "CPU_STATE_FAILURE",
            BugCheckCode::ApicFailure => "APIC_FAILURE",
            BugCheckCode::SmpFailure => "SMP_FAILURE",
            BugCheckCode::DeviceManagerFailure => "DEVICE_MANAGER_FAILURE",
            BugCheckCode::IoManagerFailure => "IO_MANAGER_FAILURE",
            BugCheckCode::StorageFailure => "STORAGE_FAILURE",
            BugCheckCode::BootloaderFailure => "BOOTLOADER_FAILURE",
            BugCheckCode::SecureBootFailure => "SECURE_BOOT_FAILURE",
            BugCheckCode::CryptoEngineFailure => "CRYPTO_ENGINE_FAILURE",
            BugCheckCode::RngFailure => "RNG_FAILURE",
            BugCheckCode::AcpiFailure => "ACPI_FAILURE",
            BugCheckCode::ShutdownFailure => "SHUTDOWN_FAILURE",
        }
    }

    pub fn error_string(&self) -> &'static str {
        match *self {
            BugCheckCode::MemoryPressure => "MEMORY_NO_KILLABLE_PROCESS",
            BugCheckCode::MemoryReclaimFailure => "MEMORY_RECLAIM_NO_PROGRESS",
            BugCheckCode::KernelHeapCorruption => "HEAP_STATE_CORRUPTED",
            BugCheckCode::KernelStackFailure => "STACK_GUARD_CORRUPTED",
            BugCheckCode::KernelInvariant => "KERNEL_INVARIANT_BROKEN",
            BugCheckCode::SchedulerFailure => "SCHEDULER_STATE_CORRUPTED",
            BugCheckCode::ProcessManagerFailure => "PROCESS_TABLE_CORRUPTED",
            BugCheckCode::InitFailure => "INIT_RECOVERY_FAILED",
            BugCheckCode::IpcFailure => "IPC_STATE_CORRUPTED",
            BugCheckCode::MemoryManagerFailure => "MEMORY_MANAGER_STATE_CORRUPTED",
            BugCheckCode::PageTableFailure => "PAGE_TABLE_STATE_CORRUPTED",
            BugCheckCode::PageFaultFatal => "PAGE_ACCESS_UNRECOVERABLE",
            BugCheckCode::DoubleFault => "FAULT_HANDLER_REENTRY_FAILURE",
            BugCheckCode::TripleFault => "CPU_EXCEPTION_UNRECOVERABLE",
            BugCheckCode::InterruptFailure => "INTERRUPT_STATE_CORRUPTED",
            BugCheckCode::IdtFailure => "IDT_STATE_CORRUPTED",
            BugCheckCode::GdtFailure => "GDT_STATE_CORRUPTED",
            BugCheckCode::SyscallFailure => "SYSCALL_GATE_CORRUPTED",
            BugCheckCode::VmSupervisorFailure => "VM_STATE_UNRECOVERABLE",
            BugCheckCode::DriverSupervisor => "DRIVER_RECOVERY_EXHAUSTED",
            BugCheckCode::GpuRecoveryFailure => "GPU_FALLBACK_UNAVAILABLE",
            BugCheckCode::DriverSandboxFailure => "DRIVER_SANDBOX_BROKEN",
            BugCheckCode::CapabilityEngineFailure => "CAPABILITY_STATE_CORRUPTED",
            BugCheckCode::SecurityInvariant => "SECURITY_POLICY_BROKEN",
            BugCheckCode::DmaProtectionFailure => "DMA_POLICY_VIOLATION",
            BugCheckCode::HardwareAbstractionFailure => "HARDWARE_STATE_UNRECOVERABLE",
            BugCheckCode::FilesystemFailure => "FILESYSTEM_STATE_CORRUPTED",
            BugCheckCode::RootfsFailure => "ROOTFS_RECOVERY_FAILED",
            BugCheckCode::KernelTtyFailure => "KERNELTTY_RECOVERY_FAILED",
            BugCheckCode::BootStateFailure => "BOOT_STATE_INVALID",
            BugCheckCode::KernelLogFailure => "KERNEL_LOGGING_UNAVAILABLE",
            BugCheckCode::KernelRecoveryFailure => "KERNEL_RECOVERY_ALL_FAILED",
            BugCheckCode::KernelStateCorruption => "KERNEL_STATE_NO_LONGER_VALID",
            BugCheckCode::SecurityBoundaryFailure => "SECURITY_BOUNDARY_COMPROMISED",
            BugCheckCode::MemoryIsolationFailure => "MEMORY_ISOLATION_COMPROMISED",
            BugCheckCode::PrivilegeBoundaryFailure => "PRIVILEGE_BOUNDARY_COMPROMISED",
            BugCheckCode::KernelCodeCorruption => "KERNEL_CODE_STATE_CORRUPTED",
            BugCheckCode::KernelMemoryCorruption => "KERNEL_MEMORY_STATE_CORRUPTED",
            BugCheckCode::TrustModelFailure => "KERNEL_TRUST_MODEL_INVALID",
            BugCheckCode::KernelCoreFailure => "KERNEL_CORE_UNRECOVERABLE",
            BugCheckCode::SystemIntegrityFailure => "SYSTEM_INTEGRITY_UNRECOVERABLE",
            BugCheckCode::IsolationFailure => "ALL_ISOLATION_LAYERS_FAILED",
            BugCheckCode::PanicHandlerFailure => "PANIC_HANDLER_SELF_FAILURE",
            BugCheckCode::TotalKernelFailure => "KERNEL_EXECUTION_UNRECOVERABLE",
            BugCheckCode::UnknownFatalFailure => "KERNEL_FAILURE_UNKNOWN",
            BugCheckCode::TimerFailure => "SYSTEM_TIMER_STATE_INVALID",
            BugCheckCode::ClockFailure => "SYSTEM_CLOCK_STATE_INVALID",
            BugCheckCode::ContextSwitchFailure => "CONTEXT_STATE_CORRUPTED",
            BugCheckCode::CpuStateFailure => "CPU_CONTEXT_UNRECOVERABLE",
            BugCheckCode::ApicFailure => "APIC_STATE_UNRECOVERABLE",
            BugCheckCode::SmpFailure => "CPU_TOPOLOGY_UNRECOVERABLE",
            BugCheckCode::DeviceManagerFailure => "DEVICE_STATE_CORRUPTED",
            BugCheckCode::IoManagerFailure => "IO_STATE_UNRECOVERABLE",
            BugCheckCode::StorageFailure => "STORAGE_STATE_UNRECOVERABLE",
            BugCheckCode::BootloaderFailure => "BOOT_HANDOFF_UNRECOVERABLE",
            BugCheckCode::SecureBootFailure => "BOOT_TRUST_UNRECOVERABLE",
            BugCheckCode::CryptoEngineFailure => "CRYPTO_STATE_UNRECOVERABLE",
            BugCheckCode::RngFailure => "RANDOM_SOURCE_UNAVAILABLE",
            BugCheckCode::AcpiFailure => "POWER_STATE_UNRECOVERABLE",
            BugCheckCode::ShutdownFailure => "SYSTEM_SHUTDOWN_UNRECOVERABLE",
        }
    }

    pub fn panic_message(&self) -> &'static str {
        match *self {
            BugCheckCode::MemoryPressure => "Everyone is important. Nobody can be sacrificed.",
            BugCheckCode::MemoryReclaimFailure => "RAM is gone. Reclaim has failed.",
            BugCheckCode::KernelHeapCorruption => "The heap is no longer a trustworthy place.",
            BugCheckCode::KernelStackFailure => "The stack has reached the fucking ceiling.",
            BugCheckCode::KernelInvariant => "Something fundamental just went horribly wrong.",
            BugCheckCode::SchedulerFailure => "The scheduler has forgotten how time works.",
            BugCheckCode::ProcessManagerFailure => "The process table has lost the plot.",
            BugCheckCode::InitFailure => "PID 1 has left the server.",
            BugCheckCode::IpcFailure => "Inter-process communication has stopped communicating.",
            BugCheckCode::MemoryManagerFailure => "The memory manager has forgotten where memory is.",
            BugCheckCode::PageTableFailure => "The MMU has lost the map.",
            BugCheckCode::PageFaultFatal => "That page was not supposed to exist.",
            BugCheckCode::DoubleFault => "One fault wasn't enough, apparently.",
            BugCheckCode::TripleFault => "The CPU has rage-quit.",
            BugCheckCode::InterruptFailure => "The interrupt system has chosen violence.",
            BugCheckCode::IdtFailure => "The IDT is no longer telling the truth.",
            BugCheckCode::GdtFailure => "The GDT has become legally questionable.",
            BugCheckCode::SyscallFailure => "The syscall gate has fallen apart.",
            BugCheckCode::VmSupervisorFailure => "The Yanase VM has entered the forbidden dimension.",
            BugCheckCode::DriverSupervisor => "The driver department has run out of ideas.",
            BugCheckCode::GpuRecoveryFailure => "Even Basic VGA couldn't save us.",
            BugCheckCode::DriverSandboxFailure => "The driver sandbox has been thoroughly violated.",
            BugCheckCode::CapabilityEngineFailure => "The kernel no longer knows who is allowed to do what.",
            BugCheckCode::SecurityInvariant => "Trust has officially left the building.",
            BugCheckCode::DmaProtectionFailure => "That DMA request was way too ambitious.",
            BugCheckCode::HardwareAbstractionFailure => "The hardware has stopped cooperating.",
            BugCheckCode::FilesystemFailure => "The filesystem has entered the danger zone.",
            BugCheckCode::RootfsFailure => "I can't find the root filesystem. Where did you put it?",
            BugCheckCode::KernelTtyFailure => "Even KernelTTY couldn't save us.",
            BugCheckCode::BootStateFailure => "We were not supposed to reach this state.",
            BugCheckCode::KernelLogFailure => "The kernel forgot how to scream for help.",
            BugCheckCode::KernelRecoveryFailure => "Every recovery mechanism has failed.",
            BugCheckCode::KernelStateCorruption => "The kernel state is no longer trustworthy.",
            BugCheckCode::SecurityBoundaryFailure => "The security boundary is gone. Stop everything.",
            BugCheckCode::MemoryIsolationFailure => "Memory isolation has failed. Nobody is safe.",
            BugCheckCode::PrivilegeBoundaryFailure => "Privilege boundaries are no longer trustworthy.",
            BugCheckCode::KernelCodeCorruption => "The kernel code itself has been compromised.",
            BugCheckCode::KernelMemoryCorruption => "The kernel's own memory has betrayed us.",
            BugCheckCode::TrustModelFailure => "The kernel can no longer trust its own reality.",
            BugCheckCode::KernelCoreFailure => "The kernel core is gone. There is nothing left to save.",
            BugCheckCode::SystemIntegrityFailure => "System integrity is gone. Pull the plug.",
            BugCheckCode::IsolationFailure => "Every sandbox is compromised. This is the end.",
            BugCheckCode::PanicHandlerFailure => "The panic handler itself has panicked. We're beyond cooked.",
            BugCheckCode::TotalKernelFailure => "Kernel execution is no longer possible.",
            BugCheckCode::UnknownFatalFailure => "Something went catastrophically wrong. Good luck.",
            BugCheckCode::TimerFailure => "Time has stopped making sense.",
            BugCheckCode::ClockFailure => "The clock has decided reality is optional.",
            BugCheckCode::ContextSwitchFailure => "The CPU forgot who it was running.",
            BugCheckCode::CpuStateFailure => "The CPU state is beyond recovery.",
            BugCheckCode::ApicFailure => "The APIC has stopped answering.",
            BugCheckCode::SmpFailure => "The CPUs are no longer agreeing on reality.",
            BugCheckCode::DeviceManagerFailure => "The device manager has lost the plot.",
            BugCheckCode::IoManagerFailure => "I/O has officially stopped making sense.",
            BugCheckCode::StorageFailure => "Storage has entered the forbidden state.",
            BugCheckCode::BootloaderFailure => "The boot process forgot how to finish.",
            BugCheckCode::SecureBootFailure => "Boot trust has left the building.",
            BugCheckCode::CryptoEngineFailure => "The kernel can no longer prove what it trusts.",
            BugCheckCode::RngFailure => "We have run out of trustworthy randomness.",
            BugCheckCode::AcpiFailure => "Power management has chosen violence.",
            BugCheckCode::ShutdownFailure => "Even shutdown is broken. Impressive.",
        }
    }
}

// ==========================================
// BUGCHECK DISPLAY
// ==========================================

/// Trigger a kernel panic with a specific bugcheck code
pub fn bugcheck(code: BugCheckCode) -> ! {
    let mut serial = crate::serial::Serial;
    
    // Draw border using ASCII characters
    let border_top    = "+======================================================================+";
    let border_bottom = "+======================================================================+";
    let separator     = "+----------------------------------------------------------------------+";
    
    // Serial output
    let _ = writeln!(&mut serial, "\n");
    let _ = writeln!(&mut serial, "{}", border_top);
    let _ = writeln!(&mut serial, "|                            KERNEL BUGCHECK                          |");
    let _ = writeln!(&mut serial, "{}", border_bottom);
    let _ = writeln!(&mut serial, "");
    let _ = writeln!(&mut serial, "  BugCheck Code : 0x{:08X}", code as u32);
    let _ = writeln!(&mut serial, "  Severity      : {}", code.severity());
    let _ = writeln!(&mut serial, "  Symbol        : {}", code.symbol());
    let _ = writeln!(&mut serial, "  Error String  : {}", code.error_string());
    let _ = writeln!(&mut serial, "");
    let _ = writeln!(&mut serial, "  Message       : {}", code.panic_message());
    let _ = writeln!(&mut serial, "");
    let _ = writeln!(&mut serial, "{}", border_top);
    let _ = writeln!(&mut serial, "|                            SYSTEM HALTED                             |");
    let _ = writeln!(&mut serial, "{}", border_bottom);
    
    // Console output (using ASCII art borders)
    crate::println!("\n");
    crate::println!("{}", border_top);
    crate::println!("|                             KERNEL BUGCHECK                          |");
    crate::println!("{}", border_bottom);
    crate::println!("");
    crate::println!("  BugCheck Code : 0x{:08X}", code as u32);
    crate::println!("  Severity      : {}", code.severity());
    crate::println!("  Symbol        : {}", code.symbol());
    crate::println!("  Error String  : {}", code.error_string());
    crate::println!("");
    crate::println!("  Message       : {}", code.panic_message());
    crate::println!("");
    crate::println!("{}", separator);
    crate::println!("|                            SYSTEM HALTED                             |");
    crate::println!("{}", border_bottom);
    
    loop {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)); }
    }
}

/// Convenience macro for bugcheck
#[macro_export]
macro_rules! bugcheck {
    ($code:ident) => {
        $crate::bugcheck::bugcheck($crate::bugcheck::BugCheckCode::$code)
    };
}

/// Check if a condition is true, if not trigger bugcheck
pub fn assert_bug(condition: bool, code: BugCheckCode) {
    if !condition {
        bugcheck(code);
    }
}

/// Macro for assert with bugcheck
#[macro_export]
macro_rules! assert_bug {
    ($cond:expr, $code:ident) => {
        if !$cond {
            $crate::bugcheck::bugcheck($crate::bugcheck::BugCheckCode::$code);
        }
    };
}