struct Pair { float x; float y = 7.0; };
struct Container { Pair pair; float3 vector; float tail = 8.0; };
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Container flat = {1.0, 2.0, 3.0, 4.0, 5.0, 6.0};
    Container nested = {{1.0}, {2.0, 3.0}, 4.0};
    float grid[2][3] = {1.0, 2.0, 3.0, 4.0};
    result[0] = flat.vector.z + flat.tail + nested.pair.y + nested.vector.z + grid[1][2];
}
