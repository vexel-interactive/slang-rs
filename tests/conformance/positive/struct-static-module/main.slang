import lib;
RWStructuredBuffer<uint> output;
static const int Count = 99;
static int value = 99;
typealias Alias = Shared;
[shader("compute")]
[numthreads(Alias.Count, 1, 1)]
void main()
{
    Shared a = {};
    int values[Shared.Count] = {1, 2, 3};
    output[0] = uint(a.read());
    Shared.add();
    output[1] = uint(a.read());
    Shared.visible += 2;
    output[2] = uint(a.visible);
    output[3] = uint(values[2]);
    output[4] = uint(Count + value);
    int pair[2] = {1, 2};
    output[5] = uint(a.index(pair));
}
