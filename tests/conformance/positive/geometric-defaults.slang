static int3 axis = int3(0, 1, 0);
int3 read(int3 value = cross(int3(1), axis)) { return value; }
RWStructuredBuffer<int3> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    int3 axis = int3(1, 0, 0);
    output[0] = read();
}
