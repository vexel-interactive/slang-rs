T copy<T>(T value) { return (T)value; }
vector<T, 2> copyPair<T>(vector<T, 2> value) { return (vector<T, 2>)value; }
StructuredBuffer<int> input;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    copy<RWStructuredBuffer<uint>>(output)[0] = uint(copy<StructuredBuffer<int>>(input)[0]);
    output[1] = uint(copyPair<int>(int2(input[1], input[2])).y);
}
