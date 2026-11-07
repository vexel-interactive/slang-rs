module bound_resources;

public Texture2D<float> image : register(t3, space5);

public float readImage()
{
    return image[uint2(0)];
}
