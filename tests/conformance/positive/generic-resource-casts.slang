struct Handle<T>
{
    T value;
    T read() { return (T)value; }
};
StructuredBuffer<int> input;
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Handle<StructuredBuffer<int>> source = {input};
    Handle<RWStructuredBuffer<uint>> destination = {output};
    destination.read()[0] = uint(source.read()[0]);
}
