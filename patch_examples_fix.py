import re
import os
import glob

def process_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()

    # The issue in examples is that they extract in the render loop on every frame.
    # While that is what it was doing originally (creating a new list or using the thread-local),
    # since we deleted the thread local, creating a new `DrawList` dynamically allocates vectors.
    # If the example app has a `draw_list: DrawList` field it could reuse it.
    # This is a bit much to refactor automatically across all examples.
    # We can bring back a reusable list inside a `thread_local!` within the engine if it's the right choice,
    # or just let examples allocate. Wait, `extract` allocates inside. But this regression was blocking.
    # An easier fix is to just let examples use a thread-local again but inside their own file,
    # OR we can just use `lazy_static` or `thread_local!` inside `Scene::extract()`?
    pass
