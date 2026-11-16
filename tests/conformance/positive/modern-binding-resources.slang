struct Data {
    float value;
}

int choose(Data value) { return 1; }
int choose(ConstantBuffer<Data> value) { return 2; }

ConstantBuffer<Data> parameters;
RWStructuredBuffer<uint> output;
Texture2D<float4> source;
SamplerState sampler;

T copy<T>(T value) {
    let handle = value;
    return handle;
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let handle = parameters;
    var value : Data = handle;
    let buffer = copy(output);
    let texture = source;
    let state = sampler;
    let pixel = texture.SampleLevel(state, float2(0), 0);
    buffer[0] = uint(choose(handle));
    buffer[1] = uint(choose(value));
    buffer[2] = uint(value.value + pixel.x);
}
