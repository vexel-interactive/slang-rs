cbuffer Parameters : register(b3) {
    var scalar : float;
    let vector = float2(2, 3);
    var matrix : float2x2;
}

RWStructuredBuffer<float> output;
[shader("compute")]
[numthreads(1, 1, 1)]
void main() {
    let copy = scalar + vector.x;
    var row = matrix[0];
    output[0] = copy + row.y;
}
