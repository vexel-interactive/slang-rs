[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let value = 2;
    value += 1;
}
