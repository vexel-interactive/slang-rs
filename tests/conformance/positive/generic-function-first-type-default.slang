struct First { int first; };
struct Second { int second; };
T create<T = First>();
U create<U = Second>() { return U(); }
RWStructuredBuffer<uint> output;
[shader("compute")][numthreads(1, 1, 1)]
void main()
{
    output[0] = uint(create().first);
    output[1] = uint(create<Second>().second);
}
