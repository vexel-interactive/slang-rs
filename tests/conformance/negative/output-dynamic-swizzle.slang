void update(inout int value) { value += 1; }
[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 thread : SV_DispatchThreadID)
{
    int2 value = int2(1, 2);
    update(value.yx[thread.x]);
}
