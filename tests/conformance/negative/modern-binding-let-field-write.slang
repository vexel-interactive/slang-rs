struct Item {
    let value = 2;
    [mutating] void write() { value = 3; }
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    var value = Item();
    value.write();
}
