T zero<T>(T value) { return T(); }
int zero(StructuredBuffer<int> value) { return 1; }

T shaped<T>(T value) { vector<T, 2> unused; return value; }
int shaped(StructuredBuffer<int> value) { return 2; }

struct Payload<T> { vector<T, 2> value; };
T nested<T>(T value) { Payload<T> unused; return value; }
int nested(StructuredBuffer<int> value) { return 3; }

int defaulted<T>(T value, T ignored = T()) { return 4; }
int defaulted(StructuredBuffer<int> value) { return 5; }

struct Holder<T>
{
    T value;
    T unused() { vector<T, 2> invalid; return value; }
    T read() { return value; }
};

StructuredBuffer<int> input;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(zero(input));
    output[1] = uint(shaped(input));
    output[2] = uint(nested(input));
    output[3] = uint(defaulted(input));
    Holder<StructuredBuffer<int>> holder = {input};
    output[4] = uint(holder.read()[0]);
}
