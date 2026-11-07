Texture2D<float4> image;

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    image[id.xy] = float4(1.0);
}
