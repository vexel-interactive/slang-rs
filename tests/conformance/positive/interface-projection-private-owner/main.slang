module main;
import lib;
struct Private : Read { float read(int input) { return float(input) + 2.75; } };
struct GenericPrivate : GenericRead<int> { float read(int input) { return float(input) + 4.75; } };
struct StaticPrivate : StaticRead { float read(int input) { return float(input) + 6.75; } };
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    Private first;
    GenericPrivate second;
    StaticPrivate third;
    output[0] = uint(first.read(1.9));
    output[1] = uint(first.read(3.9));
    output[2] = uint(second.read(1.9));
    output[3] = uint(StaticPrivate.read(third, 1.9));
}
