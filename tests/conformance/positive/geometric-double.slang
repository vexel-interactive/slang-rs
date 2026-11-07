StructuredBuffer<double4> input;
RWStructuredBuffer<double> output;
double evaluate0(double value)
{
    double size = length(value);
    double gap = distance(value, value + 1);
    double unit = normalize(value);
    double mirror = reflect(value, value);
    double bent = refract(value, value, double(0.5));
    double1 face = faceforward(value, value, value);
    return size + gap + unit + mirror + bent + face.x;
}
double evaluate1(double1 value)
{
    double size = length(value);
    double gap = distance(value, value + 1);
    double1 unit = normalize(value);
    double1 mirror = reflect(value, value);
    double1 bent = refract(value, value, double(0.5));
    double1 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
double evaluate2(double2 value)
{
    double size = length(value);
    double gap = distance(value, value + 1);
    double2 unit = normalize(value);
    double2 mirror = reflect(value, value);
    double2 bent = refract(value, value, double(0.5));
    double2 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
double evaluate3(double3 value)
{
    double size = length(value);
    double gap = distance(value, value + 1);
    double3 unit = normalize(value);
    double3 mirror = reflect(value, value);
    double3 bent = refract(value, value, double(0.5));
    double3 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
double evaluate4(double4 value)
{
    double size = length(value);
    double gap = distance(value, value + 1);
    double4 unit = normalize(value);
    double4 mirror = reflect(value, value);
    double4 bent = refract(value, value, double(0.5));
    double4 face = faceforward(value, value, value);
    return size + gap + unit.x + mirror.x + bent.x + face.x;
}
[shader("compute")]
[numthreads(1, 1, 1)]
void main()
{
    output[0] = evaluate0(double(input[0].x));
    output[1] = evaluate1(double1(input[0].x));
    output[2] = evaluate2(double2(input[0].xy));
    output[3] = evaluate3(double3(input[0].xyz));
    output[4] = evaluate4(double4(input[0].xyzw));
    output[5] = double(cross(double3(input[0].xyz), double3(input[1].xyz)).x);
}
