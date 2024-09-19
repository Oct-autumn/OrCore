# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
KERNEL_BIN := $(KERNEL_ELF).bin
DISASM_TMP := target/$(TARGET)/$(MODE)/asm

# BOARD
BOARD ?= qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin
K210_BOOTLOADER_SIZE := 131072

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
# 	k210的内存可用大小只有8M
	KERNEL_ENTRY_PA := 0x80020000	
endif

# Run K210
K210-SERIALPORT	= /dev/ttyUSB0
K210-BURNER = ../tools/kflash.py

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

# Disassembly
DISASM ?= -x

TEST ?= 0

build: env switch-check $(KERNEL_BIN)

# 切换环境时，清除上一次的编译结果
switch-check:
ifeq ($(BOARD), qemu)
	(which last-qemu) || (rm last-k210 -f && touch last-qemu && make clean)
else ifeq ($(BOARD), k210)
	(which last-k210) || (rm last-qemu -f && touch last-k210 && make clean)
endif

# 建立编译环境
env:
	(rustup target list | grep "riscv64gc-unknown-none-elf (installed)") || rustup target add $(TARGET)
	cargo install cargo-binutils --vers =0.3.3
	rustup component add rust-src
	rustup component add llvm-tools-preview

# 生成kernel.bin
$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

# 编译kernel
kernel:
#	编译user程序
	@cd ../user && make build TEST=$(TEST)
	@echo Platform: $(BOARD)
#	拷贝临时linker文件（用于内存布局）
	@cp src/linker-$(BOARD).ld src/linker.ld
#	编译kernel
	@cargo build --release --features "board_$(BOARD)"
#	删除临时linker文件
	@rm src/linker.ld

clean:
	@cargo clean

# 反汇编
disasm: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) | less

# 反汇编并用vim打开
disasm-vim: kernel
	@$(OBJDUMP) $(DISASM) $(KERNEL_ELF) > $(DISASM_TMP)
	@vim $(DISASM_TMP)
	@rm $(DISASM_TMP)

run: run-inner

# 运行
run-inner: build
# 	如果是qemu，直接运行
ifeq ($(BOARD),qemu)
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA)
else
# 	如果是k210，使用kflash.py烧录
#	下载kflash.py
	(which $(K210-BURNER)) || (cd .. && git clone https://github.com/sipeed/kflash.py.git && mv kflash.py tools)
#	bootloader.bin拷贝
	@cp $(BOOTLOADER) $(BOOTLOADER).copy
#	将kernel.bin拷贝到bootloader后面
	@dd if=$(KERNEL_BIN) of=$(BOOTLOADER).copy bs=$(K210_BOOTLOADER_SIZE) seek=1
#	生成最终bin文件
	@mv $(BOOTLOADER).copy $(KERNEL_BIN)
#	获取串口访问权限
	@sudo chmod 777 $(K210-SERIALPORT)
#	烧录
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(KERNEL_BIN)
#	打开串口通信
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200
endif

# 调试（gdb）
debug: build
	@tmux new-session -d \
		"qemu-system-riscv64 -machine virt -nographic -bios $(BOOTLOADER) -device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) -s -S" && \
		tmux split-window -h "riscv64-unknown-elf-gdb -ex 'file $(KERNEL_ELF)' -ex 'set arch riscv:rv64' -ex 'target remote localhost:1234'" && \
		tmux -2 attach-session -d

.PHONY: build env kernel clean disasm disasm-vim run-inner switch-check