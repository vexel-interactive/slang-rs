void write(out int value) { value = 3; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let value = 2;
    write(value);
}
