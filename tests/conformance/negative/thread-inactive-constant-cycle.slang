static const int width = true ? 1 : height;
static const int height = width;

[shader("compute")]
[numthreads(1, 1, 1)]
void main() {}
