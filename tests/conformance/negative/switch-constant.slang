[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    switch (id.x)
    {
        case id.y:
            break;
        default:
            break;
    }
}
