[shader("vertex")]
float4 main(uint id : sv_vertexid) : sv_position
{
    return float4(float(id), 0.0, 0.0, 1.0);
}
