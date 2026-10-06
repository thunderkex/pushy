import sys
import subprocess

def main():
    if len(sys.argv) < 2:
        sys.exit(1)
    mode = sys.argv[1]
    args = sys.argv[2:]

    cmd = ["zig", mode]

    if mode == "cc":
        cleaned_args = []
        has_target = False

        i = 0
        while i < len(args):
            arg = args[i]
            if arg.startswith("--target=") or arg.startswith("-target="):
                cleaned_args.extend(["-target", "x86_64-linux-musl"])
                has_target = True
            elif arg in ("-target", "--target") and i + 1 < len(args):
                i += 1
                cleaned_args.extend(["-target", "x86_64-linux-musl"])
                has_target = True
            else:
                cleaned_args.append(arg)
            i += 1

        if not has_target:
            cmd.extend(["-target", "x86_64-linux-musl"])

        cmd.extend(cleaned_args)
    else:
        cmd.extend(args)

    res = subprocess.run(cmd)
    sys.exit(res.returncode)

if __name__ == "__main__":
    main()
