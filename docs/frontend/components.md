# Components

## PerformanceMonitor

`src/lib/components/PerformanceMonitor.svelte`

Floating performance overlay that tracks:

- Page load time
- Time to First Byte (TTFB)
- Frames per second (FPS)
- Heap memory usage
- Component render times

### Usage

```svelte
<script>
	import PerformanceMonitor from '$lib/components/PerformanceMonitor.svelte';
	import { measureComponentRender } from '$lib/components/PerformanceMonitor.svelte';

	function loadMyData() {
		measureComponentRender('myComponent', () => {
			// your code here
		});
	}
</script>

<PerformanceMonitor />
```

### Exports

| Export                   | Type      | Description                  |
| ------------------------ | --------- | ---------------------------- |
| `PerformanceMonitor`     | Component | Floating overlay UI          |
| `measureComponentRender` | Function  | Measure a specific operation |
| `collectMetric`          | Function  | Collect a custom metric      |

### Features

- Start/stop profiling
- Export metrics as JSON
- Real-time FPS display
- Memory usage tracking
