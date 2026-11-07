static int3 direction = int3(1,2,3);
int read(int value = dot(direction, {1,0,0})) { return value; }
RWStructuredBuffer<int> output;
[shader("compute")][numthreads(1,1,1)]void main()
{
    int3 direction = int3(3,2,1);
    output[0] = read();
}
