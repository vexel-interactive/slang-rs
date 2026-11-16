module main;
import lib;
RWStructuredBuffer<uint> output;
int implementationSeed() { return 7; }
struct Value : Read {
    internal int read(int input, int extra = implementationSeed()) { return input + extra; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    int implementationSeed = 100;
    float interfaceSeed = 90.9;
    Value value;
    output[0] = uint(invoke(value));
    LibraryValue imported;
    output[1] = uint(invoke(imported));
    PrototypeValue prototype;
    output[2] = uint(invoke(prototype));
}
