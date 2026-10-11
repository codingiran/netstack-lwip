use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn sdk_include_path_for(sdk: &str) -> String {
    let output = Command::new("xcrun")
        .arg("--sdk")
        .arg(sdk)
        .arg("--show-sdk-path")
        .output()
        .expect("failed to execute xcrun");

    assert!(output.status.success(), "xcrun could not locate {sdk}");

    let inc_path = Path::new(String::from_utf8_lossy(&output.stdout).trim()).join("usr/include");

    inc_path.to_str().expect("invalid include path").to_string()
}

fn sdk_include_path() -> Option<String> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target = env::var("TARGET").unwrap();
    match os.as_str() {
        "ios" => {
            if target.ends_with("-sim") || target.starts_with("x86_64-") {
                Some(sdk_include_path_for("iphonesimulator"))
            } else {
                Some(sdk_include_path_for("iphoneos"))
            }
        }
        "macos" => Some(sdk_include_path_for("macosx")),
        "tvos" => Some(sdk_include_path_for(if target.ends_with("-sim") || target.starts_with("x86_64-") {
            "appletvsimulator"
        } else {
            "appletvos"
        })),
        _ => None,
    }
}

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
        .flag_if_supported("-Wno-everything");
    if let Some(sdk_include_path) = sdk_include_path() {
        build.include(sdk_include_path);
    }
    build.debug(true);
    build.compile("liblwip.a");
}

fn prepare_lwip_bindings() {
    println!("cargo:rerun-if-changed=src/bindings.rs");
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:include=src/lwip/include");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");
    // Preserve the tested upstream bindings unless regeneration is explicitly requested.
    // Never write generated files into the shared source tree during cross compilation.
    if env::var_os("BINDINGS_GEN").is_none() && env::var("CARGO_CFG_TARGET_OS").unwrap() != "windows" {
        std::fs::copy("src/bindings.rs", out_path).expect("failed to copy lwIP bindings");
        return;
    }

    let compiler = cc::Build::new().get_compiler();
    let mut builder = bindgen::Builder::default()
        .size_t_is_usize(false)
        .header("src/wrapper.h")
        .clang_arg("-I./src/lwip/include")
        .clang_arg("-I./src/lwip/custom")
        .clang_arg("-Wno-everything")
        .layout_tests(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    // cc-rs supplies the exact Apple target and sysroot, including ARM64 simulators.
    if env::var("CARGO_CFG_TARGET_VENDOR").unwrap() == "apple" {
        for argument in compiler.args() {
            builder = builder.clang_arg(argument.to_string_lossy());
        }
    }
    let bindings = builder.generate().expect("Unable to generate bindings");

    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
}

fn main() {
    println!("cargo:rerun-if-env-changed=BINDINGS_GEN");
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if os == "tvos" || os == "ios" || os == "android" || os == "linux" || os == "macos" || os == "windows" {
        compile_lwip();
    }

    prepare_lwip_bindings();
}
