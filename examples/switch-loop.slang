RWStructuredBuffer<int> result;
static const uint special = 1u << 2;

int classify(uint value)
{
    switch (value)
    {
        case 0u:
        case 1u:
            return 10;
        case special:
            return 20;
        default:
            return 30;
    }
}

[shader("compute")]
[numthreads(1, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    uint i = 0u;
    int total = 0;
    do
    {
        i++;
        switch (i)
        {
            case 2u:
                continue;
            case 3u:
                total += 3;
                break;
            default:
                total += classify(i);
        }
    } while (i < 4u);
    float3 values = float3(total, 0, 1);
    result[id.x] = (int)values.x;
}
