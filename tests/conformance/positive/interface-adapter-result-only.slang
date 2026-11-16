RWStructuredBuffer<uint> output;
interface IntegerRead { int read(); }
struct FloatingValue : IntegerRead {
    float read() { return 3.75; }
};
interface FloatingRead { float read(); }
struct IntegerValue : FloatingRead {
    int read() { return 5; }
};
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    FloatingValue first;
    IntegerValue second;
    output[0] = uint(first.read());
    float value = second.read();
    output[1] = asuint(value);
}
