// We need to forward routine registration from C to Rust
// to avoid the linker removing the static library.

void R_init_qsi_pulse_reader_extendr(void *dll);

void R_init_qsi_pulse_reader(void *dll) {
    R_init_qsi_pulse_reader_extendr(dll);
}
