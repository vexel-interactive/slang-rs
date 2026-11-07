StructuredBuffer<half4> input;
RWStructuredBuffer<half> output;
half evaluate0(half value)
{
    half size = length(value);
    half gap = distance(value, value + 1);
    half unit = normalize(value);
    half mirror = reflect(value, value);
    half bent = refract(value, value, half(0.5));
    half1 face = faceforward(value, value, value);
    return size + gap + unit + mirror + bent + face.x;
}
half evaluate1(half1 value)
{
    half size = length(value);
    half gap = distance(value, value + 1);
    half1 unit = normalize(value);
    half1 mirror = reflect(value, value);
    half1 bent = refract(value, value, half(0.5));
    half1 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
half evaluate2(half2 value)
{
    half size = length(value);
    half gap = distance(value, value + 1);
    half2 unit = normalize(value);
    half2 mirror = reflect(value, value);
    half2 bent = refract(value, value, half(0.5));
    half2 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
half evaluate3(half3 value)
{
    half size = length(value);
    half gap = distance(value, value + 1);
    half3 unit = normalize(value);
    half3 mirror = reflect(value, value);
    half3 bent = refract(value, value, half(0.5));
    half3 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
half evaluate4(half4 value)
{
    half size = length(value);
    half gap = distance(value, value + 1);
    half4 unit = normalize(value);
    half4 mirror = reflect(value, value);
    half4 bent = refract(value, value, half(0.5));
    half4 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(half(input[0].x));
    output[1] = evaluate1(half1(input[0].x));
    output[2] = evaluate2(half2(input[0].xy));
    output[3] = evaluate3(half3(input[0].xyz));
    output[4] = evaluate4(half4(input[0].xyzw));
    output[5] = half(cross(half3(input[0].xyz), half3(input[1].xyz)).x);
}
