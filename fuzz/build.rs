fn main() {
    cc::Build::new()
        .file("c/highwayhash.c")
        .include(".")
        .compile("highwayhash");
}
