# Details

Date : 2024-09-25 22:44:27

Directory /home/octautumn/github/OrCore

Total : 48 files,  3026 codes, 129 comments, 498 blanks, all 3653 lines

[Summary](results.md) / Details / [Diff Summary](diff.md) / [Diff Details](diff-details.md)

## Files
| filename | language | code | comment | blank | total |
| :--- | :--- | ---: | ---: | ---: | ---: |
| [os/.cargo/config.toml](/os/.cargo/config.toml) | TOML | 5 | 0 | 2 | 7 |
| [os/Cargo.toml](/os/Cargo.toml) | TOML | 19 | 2 | 5 | 26 |
| [os/build.rs](/os/build.rs) | Rust | 66 | 0 | 8 | 74 |
| [os/src/config/mod.rs](/os/src/config/mod.rs) | Rust | 23 | 0 | 8 | 31 |
| [os/src/console.rs](/os/src/console.rs) | Rust | 35 | 0 | 8 | 43 |
| [os/src/error/mem.rs](/os/src/error/mem.rs) | Rust | 9 | 0 | 1 | 10 |
| [os/src/error/mod.rs](/os/src/error/mod.rs) | Rust | 41 | 0 | 8 | 49 |
| [os/src/kernel_log.rs](/os/src/kernel_log.rs) | Rust | 49 | 0 | 7 | 56 |
| [os/src/lang_items.rs](/os/src/lang_items.rs) | Rust | 36 | 0 | 10 | 46 |
| [os/src/loader/mod.rs](/os/src/loader/mod.rs) | Rust | 66 | 0 | 12 | 78 |
| [os/src/main.rs](/os/src/main.rs) | Rust | 62 | 0 | 13 | 75 |
| [os/src/mem/address.rs](/os/src/mem/address.rs) | Rust | 256 | 0 | 47 | 303 |
| [os/src/mem/frame_allocator.rs](/os/src/mem/frame_allocator.rs) | Rust | 139 | 0 | 24 | 163 |
| [os/src/mem/heap_allocator.rs](/os/src/mem/heap_allocator.rs) | Rust | 49 | 0 | 7 | 56 |
| [os/src/mem/memory_set.rs](/os/src/mem/memory_set.rs) | Rust | 478 | 0 | 47 | 525 |
| [os/src/mem/mod.rs](/os/src/mem/mod.rs) | Rust | 30 | 0 | 7 | 37 |
| [os/src/mem/page_table.rs](/os/src/mem/page_table.rs) | Rust | 272 | 0 | 34 | 306 |
| [os/src/sbi_call.rs](/os/src/sbi_call.rs) | Rust | 56 | 0 | 10 | 66 |
| [os/src/sync/mod.rs](/os/src/sync/mod.rs) | Rust | 2 | 0 | 2 | 4 |
| [os/src/sync/ups_cell.rs](/os/src/sync/ups_cell.rs) | Rust | 21 | 0 | 6 | 27 |
| [os/src/syscall/file_sys.rs](/os/src/syscall/file_sys.rs) | Rust | 93 | 0 | 12 | 105 |
| [os/src/syscall/mod.rs](/os/src/syscall/mod.rs) | Rust | 32 | 0 | 4 | 36 |
| [os/src/syscall/process.rs](/os/src/syscall/process.rs) | Rust | 50 | 0 | 8 | 58 |
| [os/src/syscall/time.rs](/os/src/syscall/time.rs) | Rust | 21 | 0 | 6 | 27 |
| [os/src/task/context.rs](/os/src/task/context.rs) | Rust | 26 | 0 | 4 | 30 |
| [os/src/task/kernel_stack.rs](/os/src/task/kernel_stack.rs) | Rust | 63 | 0 | 8 | 71 |
| [os/src/task/manager.rs](/os/src/task/manager.rs) | Rust | 38 | 0 | 10 | 48 |
| [os/src/task/mod.rs](/os/src/task/mod.rs) | Rust | 122 | 0 | 19 | 141 |
| [os/src/task/pid.rs](/os/src/task/pid.rs) | Rust | 67 | 0 | 14 | 81 |
| [os/src/task/process.rs](/os/src/task/process.rs) | Rust | 155 | 0 | 20 | 175 |
| [os/src/task/processor.rs](/os/src/task/processor.rs) | Rust | 91 | 0 | 17 | 108 |
| [os/src/task/switch.rs](/os/src/task/switch.rs) | Rust | 12 | 0 | 4 | 16 |
| [os/src/trap/context.rs](/os/src/trap/context.rs) | Rust | 56 | 0 | 5 | 61 |
| [os/src/trap/mod.rs](/os/src/trap/mod.rs) | Rust | 121 | 0 | 13 | 134 |
| [os/src/trap/trap.rs](/os/src/trap/trap.rs) | Rust | 6 | 0 | 3 | 9 |
| [os/src/util/mod.rs](/os/src/util/mod.rs) | Rust | 1 | 0 | 1 | 2 |
| [os/src/util/time.rs](/os/src/util/time.rs) | Rust | 35 | 0 | 9 | 44 |
| [user/.cargo/config.toml](/user/.cargo/config.toml) | TOML | 8 | 0 | 2 | 10 |
| [user/Cargo.toml](/user/Cargo.toml) | TOML | 12 | 1 | 5 | 18 |
| [user/src/bin/initproc.rs](/user/src/bin/initproc.rs) | Rust | 23 | 4 | 4 | 31 |
| [user/src/bin/user_shell.rs](/user/src/bin/user_shell.rs) | Rust | 58 | 5 | 7 | 70 |
| [user/src/console.rs](/user/src/console.rs) | Rust | 35 | 5 | 11 | 51 |
| [user/src/lang_items.rs](/user/src/lang_items.rs) | Rust | 18 | 3 | 5 | 26 |
| [user/src/lib.rs](/user/src/lib.rs) | Rust | 34 | 2 | 10 | 46 |
| [user/src/syscall/fs.rs](/user/src/syscall/fs.rs) | Rust | 25 | 47 | 9 | 81 |
| [user/src/syscall/mod.rs](/user/src/syscall/mod.rs) | Rust | 43 | 0 | 5 | 48 |
| [user/src/syscall/process.rs](/user/src/syscall/process.rs) | Rust | 48 | 48 | 12 | 108 |
| [user/src/syscall/time.rs](/user/src/syscall/time.rs) | Rust | 19 | 12 | 5 | 36 |

[Summary](results.md) / Details / [Diff Summary](diff.md) / [Diff Details](diff-details.md)