struct Pair { float x; float y; };
float read(float x, int bias) { return x + bias; }
float read(Pair value, float bias) { return value.y + bias; }
float pick(float2 value, int bias) { return value.y + bias; }
float pick(float3 value, float bias) { return value.z + bias; }
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    result[0] = read({1.0, 2.0}, 0.0) + pick({3.0, 4.0}, 1);
}
