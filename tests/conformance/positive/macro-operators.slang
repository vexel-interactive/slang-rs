#define STRING_RAW(x) #x
#define STRING(x) STRING_RAW(x)
#define STAGE compute
#define CAT(a, b) a ## b
#define CALL(function, ...) function(__VA_ARGS__)

CAT(RW, StructuredBuffer)<float> result;

[shader(STRING(STAGE))]
[numthreads(32, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = CALL(max, float(id.x), 1.0);
}
