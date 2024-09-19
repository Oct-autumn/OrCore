# user/build.py
# 本脚本用于生成用户程序的链接脚本（LinkerScript）

import os

BASE_ADDR = 0x80400000  # 用户程序存放的基地址
STEP = 0x20000  # 内存对齐步长（128KB，也是用户程序的最大长度）
LINK_FILE = "src/linker.ld"  # 链接脚本文件

# 生成链接脚本
app_id = 0
apps = os.listdir("src/bin")
apps.sort()  # 按照文件名排序
for app in apps:
    app = app[: app.find(".")]  # 去掉文件后缀
    lines = []
    lines_before = []
    with open(LINK_FILE, "r") as f:
        for line in f.readlines():
            lines_before.append(line)
            line = line.replace(hex(BASE_ADDR), hex(BASE_ADDR + app_id * STEP))
            lines.append(line)
    # 写入链接脚本
    with open(LINK_FILE, "w+") as f:
        f.writelines(lines)
    # 编译用户程序
    os.system("cargo build --bin %s --release" % app)
    print("[build.py] UserApp %s start at %s" % (app, hex(BASE_ADDR + app_id * STEP)))
    # 恢复链接脚本
    with open(LINK_FILE, "w+") as f:
        f.writelines(lines_before)
    app_id += 1
