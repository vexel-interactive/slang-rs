struct Pair { float3 values = {1.0, 2.0}; float tail = 3.0; };
struct Container { Pair pair = {{4.0}, 5.0}; Pair elements[2] = {{{6.0}}, {}}; };
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    Container value = {};
    Container copy = Container(value);
    result[0] = copy.pair.values.z + copy.elements[0].tail + copy.elements[1].values.y;
}
