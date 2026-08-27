# OpenYanase Kernel BugCheck Codes

| Code | Severity | Symbolic Name | Error String | Panic Message |
|---:|:---:|---|---|---|
| `0x00000001` | HIGH | `MEMORY_PRESSURE` | `MEMORY_NO_KILLABLE_PROCESS` | `panic: "Everyone is important. Nobody can be sacrificed."` |
| `0x00000002` | HIGH | `MEMORY_RECLAIM_FAILURE` | `MEMORY_RECLAIM_NO_PROGRESS` | `panic: "RAM is gone. Reclaim has failed."` |
| `0x00000003` | HIGH | `KERNEL_HEAP_CORRUPTION` | `HEAP_STATE_CORRUPTED` | `panic: "The heap is no longer a trustworthy place."` |
| `0x00000004` | HIGH | `KERNEL_STACK_FAILURE` | `STACK_GUARD_CORRUPTED` | `panic: "The stack has reached the fucking ceiling."` |
| `0x00000005` | HIGH | `KERNEL_INVARIANT` | `KERNEL_INVARIANT_BROKEN` | `panic: "Something fundamental just went horribly wrong."` |
| `0x00000006` | HIGH | `SCHEDULER_FAILURE` | `SCHEDULER_STATE_CORRUPTED` | `panic: "The scheduler has forgotten how time works."` |
| `0x00000007` | HIGH | `PROCESS_MANAGER_FAILURE` | `PROCESS_TABLE_CORRUPTED` | `panic: "The process table has lost the plot."` |
| `0x00000008` | HIGH | `INIT_FAILURE` | `INIT_RECOVERY_FAILED` | `panic: "PID 1 has left the server."` |
| `0x00000009` | HIGH | `IPC_FAILURE` | `IPC_STATE_CORRUPTED` | `panic: "Inter-process communication has stopped communicating."` |
| `0x0000000A` | HIGH | `MEMORY_MANAGER_FAILURE` | `MEMORY_MANAGER_STATE_CORRUPTED` | `panic: "The memory manager has forgotten where memory is."` |
| `0x0000000B` | HIGH | `PAGE_TABLE_FAILURE` | `PAGE_TABLE_STATE_CORRUPTED` | `panic: "The MMU has lost the map."` |
| `0x0000000C` | HIGH | `PAGE_FAULT_FATAL` | `PAGE_ACCESS_UNRECOVERABLE` | `panic: "That page was not supposed to exist."` |
| `0x0000000D` | HIGH | `DOUBLE_FAULT` | `FAULT_HANDLER_REENTRY_FAILURE` | `panic: "One fault wasn't enough, apparently."` |
| `0x0000000E` | CRITICAL | `TRIPLE_FAULT` | `CPU_EXCEPTION_UNRECOVERABLE` | `panic: "The CPU has rage-quit."` |
| `0x0000000F` | HIGH | `INTERRUPT_FAILURE` | `INTERRUPT_STATE_CORRUPTED` | `panic: "The interrupt system has chosen violence."` |
| `0x00000010` | HIGH | `IDT_FAILURE` | `IDT_STATE_CORRUPTED` | `panic: "The IDT is no longer telling the truth."` |
| `0x00000011` | HIGH | `GDT_FAILURE` | `GDT_STATE_CORRUPTED` | `panic: "The GDT has become legally questionable."` |
| `0x00000012` | HIGH | `SYSCALL_FAILURE` | `SYSCALL_GATE_CORRUPTED` | `panic: "The syscall gate has fallen apart."` |
| `0x00000013` | CRITICAL | `VM_SUPERVISOR_FAILURE` | `VM_STATE_UNRECOVERABLE` | `panic: "The Yanase VM has entered the forbidden dimension."` |
| `0x00000014` | HIGH | `DRIVER_SUPERVISOR` | `DRIVER_RECOVERY_EXHAUSTED` | `panic: "The driver department has run out of ideas."` |
| `0x00000015` | CRITICAL | `GPU_RECOVERY_FAILURE` | `GPU_FALLBACK_UNAVAILABLE` | `panic: "Even Basic VGA couldn't save us."` |
| `0x00000016` | CRITICAL | `DRIVER_SANDBOX_FAILURE` | `DRIVER_SANDBOX_BROKEN` | `panic: "The driver sandbox has been thoroughly violated."` |
| `0x00000017` | CRITICAL | `CAPABILITY_ENGINE_FAILURE` | `CAPABILITY_STATE_CORRUPTED` | `panic: "The kernel no longer knows who is allowed to do what."` |
| `0x00000018` | CRITICAL | `SECURITY_INVARIANT` | `SECURITY_POLICY_BROKEN` | `panic: "Trust has officially left the building."` |
| `0x00000019` | CRITICAL | `DMA_PROTECTION_FAILURE` | `DMA_POLICY_VIOLATION` | `panic: "That DMA request was way too ambitious."` |
| `0x0000001A` | HIGH | `HARDWARE_ABSTRACTION_FAILURE` | `HARDWARE_STATE_UNRECOVERABLE` | `panic: "The hardware has stopped cooperating."` |
| `0x0000001B` | HIGH | `FILESYSTEM_FAILURE` | `FILESYSTEM_STATE_CORRUPTED` | `panic: "The filesystem has entered the danger zone."` |
| `0x0000001C` | CRITICAL | `ROOTFS_FAILURE` | `ROOTFS_RECOVERY_FAILED` | `panic: "I can't find the root filesystem. Where did you put it?"` |
| `0x0000001D` | CRITICAL | `KERNELTTY_FAILURE` | `KERNELTTY_RECOVERY_FAILED` | `panic: "Even KernelTTY couldn't save us."` |
| `0x0000001E` | HIGH | `BOOT_STATE_FAILURE` | `BOOT_STATE_INVALID` | `panic: "We were not supposed to reach this state."` |
| `0x0000001F` | HIGH | `KERNEL_LOG_FAILURE` | `KERNEL_LOGGING_UNAVAILABLE` | `panic: "The kernel forgot how to scream for help."` |
| `0x00000020` | CRITICAL | `KERNEL_RECOVERY_FAILURE` | `KERNEL_RECOVERY_ALL_FAILED` | `panic: "Every recovery mechanism has failed."` |
| `0x00000021` | CRITICAL | `KERNEL_STATE_CORRUPTION` | `KERNEL_STATE_NO_LONGER_VALID` | `panic: "The kernel state is no longer trustworthy."` |
| `0x00000022` | CRITICAL | `SECURITY_BOUNDARY_FAILURE` | `SECURITY_BOUNDARY_COMPROMISED` | `panic: "The security boundary is gone. Stop everything."` |
| `0x00000023` | CRITICAL | `MEMORY_ISOLATION_FAILURE` | `MEMORY_ISOLATION_COMPROMISED` | `panic: "Memory isolation has failed. Nobody is safe."` |
| `0x00000024` | CRITICAL | `PRIVILEGE_BOUNDARY_FAILURE` | `PRIVILEGE_BOUNDARY_COMPROMISED` | `panic: "Privilege boundaries are no longer trustworthy."` |
| `0x00000025` | CRITICAL | `KERNEL_CODE_CORRUPTION` | `KERNEL_CODE_STATE_CORRUPTED` | `panic: "The kernel code itself has been compromised."` |
| `0x00000026` | CRITICAL | `KERNEL_MEMORY_CORRUPTION` | `KERNEL_MEMORY_STATE_CORRUPTED` | `panic: "The kernel's own memory has betrayed us."` |
| `0x00000027` | CRITICAL | `TRUST_MODEL_FAILURE` | `KERNEL_TRUST_MODEL_INVALID` | `panic: "The kernel can no longer trust its own reality."` |
| `0x00000028` | CATASTROPHIC | `KERNEL_CORE_FAILURE` | `KERNEL_CORE_UNRECOVERABLE` | `panic: "The kernel core is gone. There is nothing left to save."` |
| `0x00000029` | CATASTROPHIC | `SYSTEM_INTEGRITY_FAILURE` | `SYSTEM_INTEGRITY_UNRECOVERABLE` | `panic: "System integrity is gone. Pull the plug."` |
| `0x0000002A` | CATASTROPHIC | `ISOLATION_FAILURE` | `ALL_ISOLATION_LAYERS_FAILED` | `panic: "Every sandbox is compromised. This is the end."` |
| `0x0000002B` | CATASTROPHIC | `PANIC_HANDLER_FAILURE` | `PANIC_HANDLER_SELF_FAILURE` | `panic: "The panic handler itself has panicked. We're beyond cooked."` |
| `0x0000002C` | CATASTROPHIC | `TOTAL_KERNEL_FAILURE` | `KERNEL_EXECUTION_UNRECOVERABLE` | `panic: "Kernel execution is no longer possible."` |
| `0x0000002D` | FATAL | `UNKNOWN_FATAL_FAILURE` | `KERNEL_FAILURE_UNKNOWN` | `panic: "Something went catastrophically wrong. Good luck."` |
| `0x0000002E` | HIGH | `TIMER_FAILURE` | `SYSTEM_TIMER_STATE_INVALID` | `panic: "Time has stopped making sense."` |
| `0x0000002F` | HIGH | `CLOCK_FAILURE` | `SYSTEM_CLOCK_STATE_INVALID` | `panic: "The clock has decided reality is optional."` |
| `0x00000030` | HIGH | `CONTEXT_SWITCH_FAILURE` | `CONTEXT_STATE_CORRUPTED` | `panic: "The CPU forgot who it was running."` |
| `0x00000031` | HIGH | `CPU_STATE_FAILURE` | `CPU_CONTEXT_UNRECOVERABLE` | `panic: "The CPU state is beyond recovery."` |
| `0x00000032` | CRITICAL | `APIC_FAILURE` | `APIC_STATE_UNRECOVERABLE` | `panic: "The APIC has stopped answering."` |
| `0x00000033` | CRITICAL | `SMP_FAILURE` | `CPU_TOPOLOGY_UNRECOVERABLE` | `panic: "The CPUs are no longer agreeing on reality."` |
| `0x00000034` | HIGH | `DEVICE_MANAGER_FAILURE` | `DEVICE_STATE_CORRUPTED` | `panic: "The device manager has lost the plot."` |
| `0x00000035` | HIGH | `IO_MANAGER_FAILURE` | `IO_STATE_UNRECOVERABLE` | `panic: "I/O has officially stopped making sense."` |
| `0x00000036` | CRITICAL | `STORAGE_FAILURE` | `STORAGE_STATE_UNRECOVERABLE` | `panic: "Storage has entered the forbidden state."` |
| `0x00000037` | HIGH | `BOOTLOADER_FAILURE` | `BOOT_HANDOFF_UNRECOVERABLE` | `panic: "The boot process forgot how to finish."` |
| `0x00000038` | CRITICAL | `SECURE_BOOT_FAILURE` | `BOOT_TRUST_UNRECOVERABLE` | `panic: "Boot trust has left the building."` |
| `0x00000039` | CRITICAL | `CRYPTO_ENGINE_FAILURE` | `CRYPTO_STATE_UNRECOVERABLE` | `panic: "The kernel can no longer prove what it trusts."` |
| `0x0000003A` | CRITICAL | `RNG_FAILURE` | `RANDOM_SOURCE_UNAVAILABLE` | `panic: "We have run out of trustworthy randomness."` |
| `0x0000003B` | HIGH | `ACPI_FAILURE` | `POWER_STATE_UNRECOVERABLE` | `panic: "Power management has chosen violence."` |
| `0x0000003C` | CRITICAL | `SHUTDOWN_FAILURE` | `SYSTEM_SHUTDOWN_UNRECOVERABLE` | `panic: "Even shutdown is broken. Impressive."` |
| `0x0000003D` | HIGH | `VIRTUAL_MEMORY_FAILURE` | `VIRTUAL_ADDRESS_SPACE_CORRUPTED` | `panic: "Virtual memory has lost the address book."` |
| `0x0000003E` | CRITICAL | `PHYSICAL_MEMORY_FAILURE` | `PHYSICAL_MEMORY_STATE_INVALID` | `panic: "Physical memory is no longer trustworthy."` |
| `0x0000003F` | HIGH | `SLAB_ALLOCATOR_FAILURE` | `SLAB_STATE_CORRUPTED` | `panic: "The allocator has forgotten how objects work."` |
| `0x00000040` | HIGH | `PAGE_ALLOCATOR_FAILURE` | `PAGE_ALLOCATOR_NO_PROGRESS` | `panic: "The page allocator has nothing left to give."` |
| `0x00000041` | CRITICAL | `TLB_FAILURE` | `TLB_STATE_UNRECOVERABLE` | `panic: "The TLB has forgotten where everything lives."` |
| `0x00000042` | CRITICAL | `CACHE_COHERENCY_FAILURE` | `CACHE_COHERENCY_STATE_BROKEN` | `panic: "The CPUs have stopped agreeing on memory."` |
| `0x00000043` | HIGH | `LOCK_MANAGER_FAILURE` | `LOCK_STATE_CORRUPTED` | `panic: "The locks have forgotten who owns what."` |
| `0x00000044` | HIGH | `DEADLOCK_DETECTED` | `KERNEL_LOCKS_NO_PROGRESS` | `panic: "Everyone is waiting for everyone. Classic."` |
| `0x00000045` | CRITICAL | `RACE_CONDITION_FAILURE` | `KERNEL_STATE_RACE_UNSAFE` | `panic: "The kernel lost a race against itself."` |
| `0x00000046` | HIGH | `WORKQUEUE_FAILURE` | `WORKQUEUE_STATE_CORRUPTED` | `panic: "The work queue has stopped doing work."` |
| `0x00000047` | HIGH | `THREAD_MANAGER_FAILURE` | `THREAD_STATE_UNRECOVERABLE` | `panic: "The thread manager has lost its threads."` |
| `0x00000048` | CRITICAL | `RING_TRANSITION_FAILURE` | `PRIVILEGE_TRANSITION_INVALID` | `panic: "The privilege transition went somewhere it shouldn't."` |
| `0x00000049` | CRITICAL | `VM_MEMORY_FAILURE` | `VM_MEMORY_BOUNDARY_BROKEN` | `panic: "The VM memory boundary has collapsed."` |
| `0x0000004A` | CRITICAL | `BYTECODE_ENGINE_FAILURE` | `BYTECODE_ENGINE_UNRECOVERABLE` | `panic: "The bytecode engine has forgotten the rules."` |
| `0x0000004B` | CRITICAL | `PACKAGE_LOADER_FAILURE` | `PACKAGE_LOAD_STATE_INVALID` | `panic: "The kernel can't safely load this package."` |
| `0x0000004C` | HIGH | `PACKAGE_CACHE_FAILURE` | `PACKAGE_CACHE_STATE_CORRUPTED` | `panic: "The package cache has become cursed."` |
| `0x0000004D` | CRITICAL | `CAPABILITY_TABLE_FAILURE` | `CAPABILITY_TABLE_CORRUPTED` | `panic: "The capability table no longer knows who is trusted."` |
| `0x0000004E` | CRITICAL | `SECURITY_CONTEXT_FAILURE` | `SECURITY_CONTEXT_CORRUPTED` | `panic: "Security context has become unreliable."` |
| `0x0000004F` | CRITICAL | `AUDIT_FAILURE` | `AUDIT_STATE_UNRECOVERABLE` | `panic: "We can no longer prove what happened."` |
| `0x00000050` | HIGH | `TRACE_BUFFER_FAILURE` | `TRACE_BUFFER_STATE_CORRUPTED` | `panic: "The breadcrumbs are gone."` |
| `0x00000051` | HIGH | `RING_BUFFER_FAILURE` | `RING_BUFFER_STATE_CORRUPTED` | `panic: "The ring buffer has gone off the rails."` |
| `0x00000052` | CRITICAL | `KERNEL_TIME_FAILURE` | `KERNEL_TIME_STATE_INVALID` | `panic: "Kernel time has stopped being real."` |
| `0x00000053` | HIGH | `SIGNAL_FAILURE` | `SIGNAL_STATE_CORRUPTED` | `panic: "The kernel signals are speaking nonsense."` |
| `0x00000054` | HIGH | `EVENT_MANAGER_FAILURE` | `EVENT_STATE_UNRECOVERABLE` | `panic: "The event system has stopped having events."` |
| `0x00000055` | HIGH | `RESOURCE_MANAGER_FAILURE` | `RESOURCE_STATE_CORRUPTED` | `panic: "The resource manager has lost track of everything."` |
| `0x00000056` | CRITICAL | `RESOURCE_LEAK_FAILURE` | `RESOURCE_RECLAIM_EXHAUSTED` | `panic: "We have leaked everything we could possibly leak."` |
| `0x00000057` | HIGH | `HANDLE_MANAGER_FAILURE` | `HANDLE_TABLE_CORRUPTED` | `panic: "The handles are no longer holding anything together."` |
| `0x00000058` | CRITICAL | `KERNEL_OBJECT_FAILURE` | `OBJECT_STATE_UNRECOVERABLE` | `panic: "A fundamental kernel object has become invalid."` |
| `0x00000059` | HIGH | `NAMESPACE_FAILURE` | `NAMESPACE_STATE_CORRUPTED` | `panic: "The kernel has lost its sense of namespace."` |
| `0x0000005A` | HIGH | `MOUNT_MANAGER_FAILURE` | `MOUNT_STATE_UNRECOVERABLE` | `panic: "The mount manager has lost the filesystem map."` |
| `0x0000005B` | HIGH | `VFS_FAILURE` | `VFS_STATE_CORRUPTED` | `panic: "The VFS has forgotten how files work."` |
| `0x0000005C` | CRITICAL | `BLOCK_IO_FAILURE` | `BLOCK_DEVICE_STATE_INVALID` | `panic: "Block I/O has become fundamentally unsafe."` |
| `0x0000005D` | CRITICAL | `NETWORK_STACK_FAILURE` | `NETWORK_STATE_UNRECOVERABLE` | `panic: "The network stack has disconnected from reality."` |
| `0x0000005E` | HIGH | `SOCKET_MANAGER_FAILURE` | `SOCKET_STATE_CORRUPTED` | `panic: "The socket manager has forgotten its sockets."` |
| `0x0000005F` | CRITICAL | `SECURITY_MONITOR_FAILURE` | `SECURITY_MONITOR_UNRECOVERABLE` | `panic: "The security monitor can no longer watch the system."` |
| `0x00000060` | CRITICAL | `SANDBOX_MANAGER_FAILURE` | `SANDBOX_STATE_UNRECOVERABLE` | `panic: "The sandbox manager has lost control."` |
| `0x00000061` | CRITICAL | `ISOLATION_MANAGER_FAILURE` | `ISOLATION_STATE_UNRECOVERABLE` | `panic: "Isolation can no longer be guaranteed."` |
| `0x00000062` | CATASTROPHIC | `KERNEL_TRUST_FAILURE` | `KERNEL_TRUST_STATE_BROKEN` | `panic: "The kernel no longer trusts itself."` |
| `0x00000063` | CATASTROPHIC | `SYSTEM_RECOVERY_FAILURE` | `SYSTEM_RECOVERY_ALL_FAILED` | `panic: "Every recovery path is dead."` |
| `0x00000064` | CATASTROPHIC | `TOTAL_SYSTEM_FAILURE` | `SYSTEM_STATE_UNRECOVERABLE` | `panic: "That's it. There is nothing left to save."` |

## GRUB2 / UEFI Platform Extension

| Code | Severity | Symbolic Name | Error String | Panic Message |
|---:|:---:|---|---|---|
| `0x00000100` | HIGH | `BOOT_INFO_FAILURE` | `BOOT_INFO_STRUCTURE_INVALID` | `panic: "GRUB gave us a boot map from another universe."` |
| `0x00000101` | CRITICAL | `BOOT_MEMORY_MAP_FAILURE` | `BOOT_MEMORY_MAP_INVALID` | `panic: "The memory map cannot be trusted."` |
| `0x00000102` | HIGH | `BOOT_FRAMEBUFFER_FAILURE` | `BOOT_FRAMEBUFFER_STATE_INVALID` | `panic: "The framebuffer handed to us is unusable."` |
| `0x00000103` | HIGH | `BOOT_MODULE_FAILURE` | `BOOT_MODULE_STATE_INVALID` | `panic: "One of the boot modules is absolutely cursed."` |
| `0x00000104` | CRITICAL | `PLATFORM_TABLE_FAILURE` | `PLATFORM_TABLE_STATE_INVALID` | `panic: "The platform description is no longer trustworthy."` |
| `0x00000105` | HIGH | `RUNTIME_VARIABLE_FAILURE` | `RUNTIME_VARIABLE_STATE_INVALID` | `panic: "Firmware variables have stopped cooperating."` |
| `0x00000106` | CRITICAL | `RUNTIME_MAPPING_FAILURE` | `RUNTIME_MAPPING_STATE_BROKEN` | `panic: "Runtime memory is no longer where we expected it."` |
| `0x00000107` | HIGH | `RUNTIME_TIME_FAILURE` | `RUNTIME_TIME_STATE_INVALID` | `panic: "Firmware time has stopped making sense."` |
| `0x00000108` | CRITICAL | `RUNTIME_RESET_FAILURE` | `RUNTIME_RESET_SERVICE_FAILED` | `panic: "We asked firmware to reset. Firmware refused."` |
| `0x00000109` | CRITICAL | `ACPI_PLATFORM_FAILURE` | `ACPI_PLATFORM_STATE_INVALID` | `panic: "ACPI handed the kernel a broken map of the machine."` |
| `0x0000010A` | HIGH | `SMBIOS_PLATFORM_FAILURE` | `SMBIOS_PLATFORM_STATE_INVALID` | `panic: "SMBIOS is telling us things that cannot be true."` |
| `0x0000010B` | CRITICAL | `FIRMWARE_INTERFACE_FAILURE` | `FIRMWARE_INTERFACE_STATE_BROKEN` | `panic: "The firmware interface is no longer trustworthy."` |
| `0x0000010C` | CATASTROPHIC | `PLATFORM_STATE_FAILURE` | `PLATFORM_STATE_UNRECOVERABLE` | `panic: "The platform state is beyond recovery."` |