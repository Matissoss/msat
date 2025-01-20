def msat_global_build():
    rust_targets = [
            "x86_64-unknown-linux-gnu",
            "x86_64-unknown-linux-musl",
            "x86_64-pc-windows-gnu"
    ]
    export_targets = [
            "linx86_64-libc",
            "linx86_64-musl",
            "winx86_64"
    ]
    return

print("""
====
This is official build.py script
====
| Choose build options:
| - 1: global msat build
| - 2: global msat build""")
args = input("[input]>")
if args == "1":
    msat_global_build()
elif args == "2":
    print("argument 2")
