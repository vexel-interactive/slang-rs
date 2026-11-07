static const int width = height;
static const int height = width;

[shader("compute")]
[numthreads(width, 1, 1)]
void main() {}
