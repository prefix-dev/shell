> PIXI_HOME="~/.local"
> case "$PIXI_HOME" in '~' | '~'/*) echo "tilde match" ;; *) echo "no match" ;; esac
tilde match

> PLATFORM="Darwin"
> case "$PLATFORM" in 'Darwin') PLATFORM="apple-darwin" ;; 'Linux') PLATFORM="unknown-linux-musl" ;; esac
> echo "$PLATFORM"
apple-darwin

> ARCH="arm64"
> case "${ARCH-}" in arm64 | aarch64) ARCH="aarch64" ;; riscv64) ARCH="riscv64gc" ;; esac
> echo "$ARCH"
aarch64

> _ostype="Linux"
> _clibtype="gnu"
> case "$_ostype" in
>     Android)
>         _ostype=linux-android
>         ;;
>     Linux)
>         _ostype=unknown-linux-$_clibtype
>         ;;
>     FreeBSD)
>         _ostype=unknown-freebsd
>         ;;
>     Darwin)
>         _ostype=apple-darwin
>         ;;
>     *)
>         _ostype=unknown
>         ;;
> esac
> echo "$_ostype"
unknown-linux-gnu

> _cputype="x86_64"
> case "$_cputype" in
>     i386 | i486 | i686 | i786 | x86)
>         _cputype=i686
>         ;;
>     aarch64 | arm64)
>         _cputype=aarch64
>         ;;
>     x86_64 | x86-64 | x64 | amd64)
>         _cputype=x86_64
>         ;;
>     ppc64le)
>         _cputype=powerpc64le
>         ;;
>     *)
>         _cputype=unknown
>         ;;
> esac
> echo "$_cputype"
x86_64

> uname_r="5.15.0-microsoft-standard"
> case "$uname_r" in *microsoft*) echo "WSL 2" ;; *Microsoft*) echo "WSL 1" ;; *) echo "native" ;; esac
WSL 2

> uname_r="5.15.0-plain-kernel"
> case "$uname_r" in *microsoft*) echo "WSL 2" ;; *Microsoft*) echo "WSL 1" ;; *) echo "native" ;; esac
native

> CHANNEL="stable"
> case "$CHANNEL" in stable|test) echo "valid" ;; *) echo "invalid" ;; esac
valid

> mirror="Aliyun"
> case "$mirror" in
>     Aliyun)
>         DOWNLOAD_URL="https://mirrors.aliyun.com/docker-ce"
>         ;;
>     AzureChinaCloud)
>         DOWNLOAD_URL="https://mirror.azure.cn/docker-ce"
>         ;;
>     "")
>         DOWNLOAD_URL="https://download.docker.com"
>         ;;
>     *)
>         echo "unknown mirror"
>         ;;
> esac
> echo "$DOWNLOAD_URL"
https://mirrors.aliyun.com/docker-ce

> dist="debian"
> dist_version="12"
> case "$dist_version" in 13) dist_version="trixie" ;; 12) dist_version="bookworm" ;; 11) dist_version="bullseye" ;; *) dist_version="unknown" ;; esac
> echo "$dist_version"
bookworm

> TEST_PROFILE="/home/user/.bashrc"
> case "${TEST_PROFILE-}" in *"/.bashrc" | *"/.bash_profile" | *"/.zshrc" | *"/.zprofile") echo "known" ;; *) echo "unknown" ;; esac
known

> arg="--quiet"
> case "$arg" in --help) echo "help" ;; --quiet) echo "quiet" ;; *) echo "other" ;; esac
quiet

> SHELL_NAME="bash"
> case "$SHELL_NAME" in bash) echo "bashrc" ;; fish) echo "config.fish" ;; zsh) echo "zshrc" ;; '') echo "unknown" ;; *) echo "unsupported" ;; esac
bashrc

> TERM="xterm-256color"
> case "$TERM" in xterm*|rxvt*|linux*|vt*) echo "ansi" ;; *) echo "no-ansi" ;; esac
ansi
