# AIV Workshops

To contribute make some containers and a "docker-compose.yml" that connects them and spins up a website.

1. Aside from LLMs, each workshop has to be a self contained set of containers.
2. The containers need to serve a single website.
3. GPU memory access is limited as we only have 96GB of vram in the portable cluster.

## Keep in Mind

For 90% of the participants, they will not learn the skill from your workshop. These are for people who drop in and there's a lot of other things going on at hacker cons. 

Them not learning the skill is *fine*, your task is to get them excited to learn and to guide them when they do. Prepare them for learning. You have succeeded if they leave knowing why this is important and wanting to know more.

Instilling a joy for the topic is far more difficult than a technical demo. You need to think about how much they need to invest before they get something out. This initial payoff can be as simple as your UI being pretty. It can also be your personal enthusiasm, but this doesn't "scale". 

The best demos/workshops are usually not technically interesting. They are accessible, they speak to people respectfully and they have a deep subject behind them. You studied that deep subject for a reason, show off that joy. 

## Deployment

This is deployed with a rust system that spins up the user pod per user and then uses pingora to proxy them over to their pod, identifying the user with a cookie. It is not designed to be secure, it's designed to deliver them your container with the least friction possible. 

You have **2 pods** to work with a user pod that is deployed per user, and a central pod that all users share. The per-user pod can hold state in the container (see the email-indirect user container that uses a python list for the "emails"). You should not need a database or other layers in your app. Claude/Gemini is very good at vibing front end demos with self contained state. 

## LLMs

The LLMs need to be hosted separately and we can only host a limited number of them. If you need one you should depend on 2 environment variables:

```
VLLM_URL = os.environ.get("VLLM_URL", "http://llm-services.local/v1/")
VLLM_MODEL = os.environ.get("VLLM_MODEL", "vllm-model")
```

We'll add a per-user API key for management and budget if users end up abusing their access. For testing/developement you should point the URL to a locally hosted ollama or vllm.  
