import lib;
struct Private { int value; };
Marker create<T>(T input) { return Marker(input, 3); }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    Private input = {2};
    output[0] = uint(read(Marker(input, 5)));
    output[1] = uint(read(create(input)));
}
