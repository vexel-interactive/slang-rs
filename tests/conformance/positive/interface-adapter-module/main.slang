module main;
import lib;
RWStructuredBuffer<uint> output;
float seed() { return 90.9; }
struct Value : Read {
    internal override float read(int input) { return float(input) + 4.75; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Value value;
    output[0] = uint(invoke(value));
    output[1] = uint(invoke<Value>(value));
    LibraryValue imported;
    output[2] = uint(invoke(imported));
}
