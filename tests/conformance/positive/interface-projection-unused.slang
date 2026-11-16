interface Read { int read(float input); }
struct Value<A> : Read {
    int read(int input) { return input + 7; }
}
int unused<A>(Value<A> value) { return value.read(1.0); }
RWStructuredBuffer<uint> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() { output[0] = 0; }
