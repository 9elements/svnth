# Oscillators

You can learn a lot about oscillators in this YouTube video:

https://www.youtube.com/watch?v=OSCzKOqtgcA&list=PLOjb97RpNDtU0_3hGpF9nmHVOkzsMOsws&index=5&t=0s

Here is the extract:

## Sin

(time * frequency * PI * 2.0).sin()

## Square

if (time * frequency * PI * 2.0).sin() > 0.0 {
  return 1.0;
} else {
  return -1.0;
}

## Triangle

