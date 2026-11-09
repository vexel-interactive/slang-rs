struct Payload<T>
{
    vector<T, 2> value;
    static StructuredBuffer<int> unused;
};
int choose<T>(T value) { return 1; }
int choose<T, U = int>(T value, Payload<T> extra = Payload<T>()) { return 2; }

StructuredBuffer<int> input;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main() { output[0] = uint(choose(input)); }
