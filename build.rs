use std::{
    env,
    path::{Path, PathBuf},
};

fn compile_lwip() {
    println!("cargo:rerun-if-changed=src/lwip");
    let mut build = cc::Build::new();
    build
        .file("src/lwip/core/init.c")
        .file("src/lwip/core/def.c")
        // .file("src/lwip/core/dns.c")
        .file("src/lwip/core/inet_chksum.c")
        .file("src/lwip/core/ip.c")
        .file("src/lwip/core/mem.c")
        .file("src/lwip/core/memp.c")
        .file("src/lwip/core/netif.c")
        .file("src/lwip/core/pbuf.c")
        .file("src/lwip/core/raw.c")
        // .file("src/lwip/core/stats.c")
        // .file("src/lwip/core/sys.c")
        .file("src/lwip/core/tcp.c")
        .file("src/lwip/core/tcp_in.c")
        .file("src/lwip/core/tcp_out.c")
        .file("src/lwip/core/timeouts.c")
        .file("src/lwip/core/udp.c")
        // .file("src/lwip/core/ipv4/autoip.c")
        // .file("src/lwip/core/ipv4/dhcp.c")
        // .file("src/lwip/core/ipv4/etharp.c")
        .file("src/lwip/core/ipv4/icmp.c")
        // .file("src/lwip/core/ipv4/igmp.c")
        .file("src/lwip/core/ipv4/ip4_frag.c")
        .file("src/lwip/core/ipv4/ip4.c")
        .file("src/lwip/core/ipv4/ip4_addr.c")
        // .file("src/lwip/core/ipv6/dhcp6.c")
        // .file("src/lwip/core/ipv6/ethip6.c")
        .file("src/lwip/core/ipv6/icmp6.c")
        // .file("src/lwip/core/ipv6/inet6.c")
        .file("src/lwip/core/ipv6/ip6.c")
        .file("src/lwip/core/ipv6/ip6_addr.c")
        .file("src/lwip/core/ipv6/ip6_frag.c")
        // .file("src/lwip/core/ipv6/mld6.c")
        .file("src/lwip/core/ipv6/nd6.c")
        .file("src/lwip/custom/sys_arch.c")
        .include("src/lwip/custom")
        .include("src/lwip/include")
        .warnings(false)
        .flag_if_supported("-Wno-everything")
        // Keep ram_heap (a 2MB tentative definition in mem.c) out of Mach-O
        // common symbols; ld would otherwise infer a 32KB alignment for it and
        // warn about exceeding the 16KB segment alignment on Apple Silicon.
        .flag_if_supported("-fno-common");
    build.debug(true);
    build.compile("liblwip.a");
}

fn generate_lwip_bindings(out_path: &Path) {
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:include=src/lwip/include");

    let target = env::var("TARGET").unwrap();
    let mut builder = bindgen::Builder::default()
        .header("src/wrapper.h")
        // Avoid importing unrelated C runtime symbols from transitive system
        // headers; keep lwIP declarations and their referenced types.
        .allowlist_file(r".*[/\\]lwip[/\\](include|custom)[/\\].*")
        .size_t_is_usize(false)
        .clang_arg("-I./src/lwip/include")
        .clang_arg("-I./src/lwip/custom")
        .clang_arg("-Wno-everything")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    // Use the same target, deployment version and SDK as the C compilation.
    // cc-rs resolves tvOS/iOS device and simulator triples, including ARM64
    // simulators; passing the Rust triple or just usr/include loses that ABI.
    if env::var("CARGO_CFG_TARGET_VENDOR").unwrap() == "apple" {
        for argument in cc::Build::new().get_compiler().args() {
            builder = builder.clang_arg(argument.to_string_lossy());
        }
    } else {
        builder = builder.clang_arg(format!("--target={}", target));
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn main() {
    // Generate bindings for the current target on every build (into OUT_DIR, not
    // the source tree) so the ABI always matches the target being compiled for.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    generate_lwip_bindings(&out_path);
    compile_lwip();
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // sys_win_rand uses the Windows system-preferred cryptographic RNG.
        println!("cargo:rustc-link-lib=bcrypt");
    }
}
