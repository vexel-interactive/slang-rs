struct ColorOutput
{
    float4 color : SV_Target0;
    float depth : SV_Depth;
};

[shader("fragment")]
void main(float4 position : SV_Position, out ColorOutput output)
{
    output.color = float4(position.xy, 0.0, 1.0);
    output.depth = position.z;
}
