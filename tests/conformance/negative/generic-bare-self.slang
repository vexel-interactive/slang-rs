struct Box<T>{T value;Box copy(){return this;}};[shader("compute")][numthreads(1,1,1)]void main(){Box<int> value={3};}
